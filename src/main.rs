mod actor;
use crate::actor::ActorHandle;
use tokio::runtime::Runtime;

fn main() {
    let runtime = Runtime::new().unwrap();
    let r = runtime.block_on(async {
        let a = ActorHandle::new();
        let r = a.get_unique_id().await;
        r
    });
    println!("response: {}", r);
}
