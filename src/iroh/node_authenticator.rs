use std::sync::Arc;
use std::future::Future;
use std::pin::Pin;
use iroh::NodeId;
use tokio::sync::RwLock;

pub trait NodeAuthenticator: Send + Sync + std::fmt::Debug {
    fn should_accept(&self, node_id: &NodeId) -> Pin<Box<dyn Future<Output = bool> + Send + '_>>;
}

#[derive(Debug, Clone)]
pub struct AllowAllNodeAuthenticator;

impl AllowAllNodeAuthenticator {
    pub fn new() -> Self {
        Self
    }
    
    pub fn arc() -> Arc<dyn NodeAuthenticator> {
        Arc::new(Self::new())
    }
}

impl NodeAuthenticator for AllowAllNodeAuthenticator {
    fn should_accept(&self, _node_id: &NodeId) -> Pin<Box<dyn Future<Output = bool> + Send + '_>> {
        Box::pin(async move { true })
    }
}

impl Default for AllowAllNodeAuthenticator {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct DynamicNodeAuthenticator {
    allowed_nodes: Arc<RwLock<Vec<NodeId>>>,
}

impl DynamicNodeAuthenticator {
    pub fn new(allowed_nodes: Vec<NodeId>) -> Self {
        Self {
            allowed_nodes: Arc::new(RwLock::new(allowed_nodes)),
        }
    }
    
    pub fn arc(allowed_nodes: Vec<NodeId>) -> Arc<dyn NodeAuthenticator> {
        Arc::new(Self::new(allowed_nodes))
    }
    
    pub async fn add_node(&self, node_id: NodeId) {
        let mut nodes = self.allowed_nodes.write().await;
        if !nodes.contains(&node_id) {
            nodes.push(node_id);
        }
    }
    
    pub async fn remove_node(&self, node_id: &NodeId) {
        let mut nodes = self.allowed_nodes.write().await;
        nodes.retain(|id| id != node_id);
    }
    
    pub async fn set_allowed_nodes(&self, allowed_nodes: Vec<NodeId>) {
        let mut nodes = self.allowed_nodes.write().await;
        *nodes = allowed_nodes;
    }
    
    pub async fn get_allowed_nodes(&self) -> Vec<NodeId> {
        self.allowed_nodes.read().await.clone()
    }
}

impl Clone for DynamicNodeAuthenticator {
    fn clone(&self) -> Self {
        Self {
            allowed_nodes: Arc::clone(&self.allowed_nodes),
        }
    }
}

impl NodeAuthenticator for DynamicNodeAuthenticator {
    fn should_accept(&self, node_id: &NodeId) -> Pin<Box<dyn Future<Output = bool> + Send + '_>> {
        let allowed_nodes = Arc::clone(&self.allowed_nodes);
        let node_id = *node_id;
        Box::pin(async move {
            let nodes = allowed_nodes.read().await;
            nodes.contains(&node_id)
        })
    }
}