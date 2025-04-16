use anyhow::Result;
use futures_util::Stream;
use std::{collections::HashMap, time::Duration};
use yellowstone_grpc_client::{ClientTlsConfig, GeyserGrpcClient};
use yellowstone_grpc_proto::{
    geyser::{
        CommitmentLevel, SubscribeRequest, SubscribeRequestFilterAccounts,
        SubscribeRequestFilterBlocks, SubscribeRequestFilterBlocksMeta,
        SubscribeRequestFilterTransactions, SubscribeUpdate,
    },
    tonic::Status,
};

/// transactions filter map 
type TransactionsFilterMap = HashMap<String, SubscribeRequestFilterTransactions>;

/// blocks filter map
type BlocksFilterMap = HashMap<String, SubscribeRequestFilterBlocks>;

/// account filter map
type AccountFilterMap = HashMap<String, SubscribeRequestFilterAccounts>;
/// blockhash filter map
type BlockMetaFilterMap = HashMap<String, SubscribeRequestFilterBlocksMeta>;

/// grpc structure, parameters only url
pub struct GrpcClient {
    endpoint: String,
}

impl GrpcClient {
    pub fn new(endpoint: String) -> Self {
        Self { endpoint }
    }

    /// 订阅区块
    /// Subscribe block
    pub async fn subscribe_block(
        &self,
        account_include: Vec<String>,       // 关注的地址, include addresses
        include_transactions: Option<bool>, // 是否包含所有交易, whether to include all transactions
        include_accounts: Option<bool>,     // 是否包含所有账户更新, whether to include all account updates
        include_entries: Option<bool>,      // 默认false, default false
    ) -> Result<impl Stream<Item = Result<SubscribeUpdate, Status>>> {
        // 创建client
        let mut client = GeyserGrpcClient::build_from_shared(self.endpoint.clone())?
            .tls_config(ClientTlsConfig::new().with_native_roots())?
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(60))
            .connect()
            .await?;
        // 过滤规则
        let mut blocks: BlocksFilterMap = HashMap::new();
        blocks.insert(
            "client".to_owned(),
            SubscribeRequestFilterBlocks {
                account_include,
                include_transactions,
                include_accounts,
                include_entries,
            },
        );

        // 构建request
        // build request
        let subscribe_request = SubscribeRequest {
            blocks,
            commitment: Some(CommitmentLevel::Confirmed.into()),
            ..Default::default()
        };

        // 返回流
        // return stream
        let (_, stream) = client
            .subscribe_with_request(Some(subscribe_request))
            .await?;
        Ok(stream)
    }

    /// 订阅指定地址的账户信息更新
    /// Subscribe transaction
    pub async fn subscribe_transaction(
        &self,
        account_include: Vec<String>,  // 包含在内的地址相关交易都会收到, include addresses
        account_exclude: Vec<String>,  // 不包含这些地址的相关交易都会收到, exclude addresses
        account_required: Vec<String>, // 必须要包含的地址, required addresses
        commitment: CommitmentLevel,   // 确认级别, commitment level
    ) -> Result<impl Stream<Item = Result<SubscribeUpdate, Status>>> {
        // client
        let mut client = GeyserGrpcClient::build_from_shared(self.endpoint.clone())?
            .tls_config(ClientTlsConfig::new().with_native_roots())?
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(60))
            .connect()
            .await?;

        // filter rules
        let mut transactions: TransactionsFilterMap = HashMap::new();
        transactions.insert(
            "client".to_string(),
            SubscribeRequestFilterTransactions {
                vote: None,
                failed: None,
                signature: None,
                account_include,
                account_exclude,
                account_required,
            },
        );

        let mut metas: BlockMetaFilterMap = HashMap::new();
        metas.insert("client".to_string(), SubscribeRequestFilterBlocksMeta {});
        // request
        let subscribe_request = SubscribeRequest {
            transactions,
            blocks_meta: metas,
            commitment: Some(commitment.into()),
            ..Default::default()
        };

        let (_, stream) = client
            .subscribe_with_request(Some(subscribe_request))
            .await?;

        Ok(stream)
    }

    pub async fn subscribe_account_updates(
        &self,
        account: Vec<String>,
        commitment: CommitmentLevel,
    ) -> Result<impl Stream<Item = Result<SubscribeUpdate, Status>>> {
        // client
        let mut client = GeyserGrpcClient::build_from_shared(self.endpoint.clone())?
            .tls_config(ClientTlsConfig::new().with_native_roots())?
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(60))
            .connect()
            .await?;

        // filter rules
        let mut accounts: AccountFilterMap = HashMap::new();
        accounts.insert(
            "client".to_string(),
            SubscribeRequestFilterAccounts {
                account,
                owner: vec![],
                filters: vec![],
                nonempty_txn_signature: None,
            },
        );

        // request
        let subscribe_request = SubscribeRequest {
            accounts,
            commitment: Some(commitment.into()),
            ..Default::default()
        };

        // return stream
        let (_, stream) = client
            .subscribe_with_request(Some(subscribe_request))
            .await?;

        Ok(stream)
    }

    /// Get latest blockhash
    pub async fn get_latest_blockhash(&self) -> Result<String> {
        let mut client = GeyserGrpcClient::build_from_shared(self.endpoint.clone())?
            .tls_config(ClientTlsConfig::new().with_native_roots())?
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(60))
            .connect()
            .await?;
        let response = client.get_latest_blockhash(None).await?;
        Ok(response.blockhash)
    }
}
