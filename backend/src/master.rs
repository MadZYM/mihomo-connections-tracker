use std::error::Error;
use std::sync::{Arc, Mutex};
use tokio::signal;
use crate::config::MasterConfig;
use crate::db::Database;
use crate::api::{MasterServer, MihomoClient};
use crate::common::{ConnectionState, process_connections};

// 主节点状态
struct MasterState {
    db: Arc<Database>,
    conn_state: Arc<Mutex<ConnectionState>>,
}

// 运行主节点服务器
pub async fn run(config: MasterConfig) -> Result<(), Box<dyn Error>> {
    println!("初始化主节点数据库...");
    let database = Arc::new(Database::new(&config.database).await?);
    
    println!("配置信息:");
    println!("  - 数据库: {}", config.database);
    println!("  - API监听地址: {}:{}", config.listen_host, config.listen_port);
    println!("  - API认证: {}", config.api_token.as_ref().map_or("未启用", |_| "已启用"));
    println!("  - Dashboard: http://{}:{}/ (无认证)", config.dashboard_host, config.dashboard_port);
    
    // 如果配置了Mihomo连接信息，则显示
    if let Some(mihomo_host) = &config.mihomo_host {
        println!("  - 本地Mihomo API: {}:{}", mihomo_host, config.mihomo_port.unwrap_or(9090));
        println!("  - Mihomo API认证: {}", config.mihomo_token.as_ref().map_or("未启用", |_| "已启用"));
    }
    
    // 创建主节点状态
    let conn_state = Arc::new(Mutex::new(ConnectionState::new()));
    let state = Arc::new(MasterState {
        db: database.clone(),
        conn_state: conn_state.clone(),
    });
    
    // 如果配置了本地Mihomo API，则启动本地数据收集
    let mihomo_task = if let Some(mihomo_host) = &config.mihomo_host {
        let mihomo_port = config.mihomo_port.unwrap_or(9090);
        let mihomo_token = config.mihomo_token.clone().unwrap_or_default();
        let mihomo_client = MihomoClient::new(
            mihomo_host.clone(),
            mihomo_port,
            mihomo_token
        );
        
        // 克隆状态用于Mihomo API任务
        let mihomo_state = state.clone();
        
        println!("启动本地Mihomo连接数据收集...");
        
        // 运行Mihomo数据收集任务
        Some(tokio::spawn(async move {
            if let Err(e) = run_mihomo_collection(mihomo_client, mihomo_state).await {
                eprintln!("Mihomo数据收集错误: {}", e);
            }
        }))
    } else {
        None
    };

    // Dashboard 独立监听端口，固定不启用认证。
    // 这样可以保留管理 API 的 Bearer Token，同时让可信 LAN/ZeroTier 客户端直接浏览统计页面。
    let dashboard_db = database.clone();
    let dashboard_host = config.dashboard_host.clone();
    let dashboard_port = config.dashboard_port;
    let dashboard_task = tokio::spawn(async move {
        if let Err(e) = crate::dashboard::start(dashboard_db, &dashboard_host, dashboard_port).await {
            eprintln!("Dashboard错误: {}", e);
        }
    });
    
    // 创建并启动API服务器
    let server = MasterServer::new(database, config.api_token);
    
    println!("主节点服务器已启动");
    println!("按Ctrl+C关闭服务器");
    
    // 启动服务器并等待中断信号
    tokio::select! {
        result = server.start(&config.listen_host, config.listen_port) => {
            dashboard_task.abort();
            if let Err(e) = result {
                eprintln!("服务器错误: {:?}", e);
                return Err(e);
            }
            Ok(())
        }
        _ = signal::ctrl_c() => {
            // 如果存在Mihomo任务，等待它结束
            if let Some(mihomo_handle) = mihomo_task {
                mihomo_handle.abort();
            }
            dashboard_task.abort();
            println!("收到关闭信号，正在关闭服务器...");
            Ok(())
        }
    }
}

// 运行本地Mihomo数据收集
async fn run_mihomo_collection(mihomo_client: MihomoClient, state: Arc<MasterState>) -> Result<(), Box<dyn Error>> {
    // 本地数据使用"local"作为agent_id
    let agent_id = Some("local".to_string());
    
    // 运行Mihomo客户端连接
    mihomo_client.connect(move |data| {
        // 获取数据库和当前状态
        let db = &state.db;
        
        // 获取并锁定当前状态
        let mut state_lock = match state.conn_state.lock() {
            Ok(lock) => lock,
            Err(_) => return Err("无法获取状态锁".to_string()),
        };
        
        // 使用公共函数处理连接更新
        process_connections(&data, &mut state_lock, db.clone(), agent_id.clone())
    }).await
}
