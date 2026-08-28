use clap::Args;

// 主节点配置
#[derive(Args, Debug)]
pub struct MasterConfig {
    /// 数据库路径
    #[clap(long, default_value = "master.db")]
    pub database: String,

    /// API 监听地址
    #[clap(long, default_value = "0.0.0.0")]
    pub listen_host: String,

    /// API 监听端口
    #[clap(long, default_value = "8080")]
    pub listen_port: u16,

    /// API 认证令牌
    #[clap(long)]
    pub api_token: Option<String>,

    /// Web Dashboard 监听地址。默认监听全部接口，LAN/ZeroTier 客户端可直接访问。
    #[clap(long, default_value = "0.0.0.0")]
    pub dashboard_host: String,

    /// Web Dashboard 监听端口。Dashboard 固定不启用认证。
    #[clap(long, default_value = "9092")]
    pub dashboard_port: u16,
    
    /// Mihomo API服务器主机地址（可选，用于主节点独立运行）
    #[clap(long)]
    pub mihomo_host: Option<String>,

    /// Mihomo API服务器端口号（可选，用于主节点独立运行）
    #[clap(long)]
    pub mihomo_port: Option<u16>,

    /// Mihomo API认证令牌（可选，用于主节点独立运行）
    #[clap(long)]
    pub mihomo_token: Option<String>,
}

// 从节点配置
#[derive(Args, Debug)]
pub struct AgentConfig {
    /// Mihomo API服务器主机地址
    #[clap(long, default_value = "127.0.0.1")]
    pub mihomo_host: String,

    /// Mihomo API服务器端口号
    #[clap(long, default_value = "9090")]
    pub mihomo_port: u16,

    /// Mihomo API认证令牌
    #[clap(long, default_value = "")]
    pub mihomo_token: String,

    /// 主节点地址
    #[clap(long)]
    pub master_url: Option<String>,

    /// 主节点认证令牌
    #[clap(long)]
    pub master_token: Option<String>,

    /// 本地数据库路径（离线模式使用）
    #[clap(long, default_value = "agent.db")]
    pub local_database: String,

    /// 同步间隔（秒）
    #[clap(long, default_value = "60")]
    pub sync_interval: u64,
    
    /// 节点标识符
    #[clap(long)]
    pub agent_id: Option<String>,
    
    /// 数据保留天数（超过这个天数的已同步数据会被删除）
    #[clap(long, default_value = "7")]
    pub data_retention_days: i64,
}
