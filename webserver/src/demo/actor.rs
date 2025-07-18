use tokio::sync::{ oneshot, mpsc };
use tokio;

struct TestActor {
    receiver: mpsc::Receiver<TestActorMessage>,
    next_id: u32,
}
enum TestActorMessage {
    GetUniqueId { respond_to: oneshot::Sender<u32> }
}

impl TestActor {
    fn new(receiver: mpsc::Receiver<TestActorMessage>) -> Self {
        TestActor {
            receiver,
            next_id: 0
        }
    }

    fn handle_message(&mut self, msg: TestActorMessage) {
        match msg {
            TestActorMessage::GetUniqueId { respond_to } => {
                self.next_id += 1;
                let _ = respond_to.send(self.next_id);
            }
        }
    }
}


async fn run_test_actor(mut actor: TestActor) {
    while let Some(msg) = actor.receiver.recv().await {
        actor.handle_message(msg);
    }
}

#[derive(Clone)]
struct TestActorHandle {
    sender: mpsc::Sender<TestActorMessage>,
}

impl TestActorHandle {
    fn new() -> Self {
        let (sender, receiver) = mpsc::channel(100);
        let actor = TestActor::new(receiver);
        tokio::spawn(run_test_actor(actor));
        Self { sender }
    }

    async fn get_unique_id(&self) -> u32 {
        let (send, recv) = oneshot::channel();
        let actor_message = TestActorMessage::GetUniqueId {
            respond_to: send
        };

        let _ = self.sender.send(actor_message).await;
        recv.await.expect("Actor task has been killed")
    }
}

pub async fn actor_demo()
{
    let handler = TestActorHandle::new();
    let handler_1 = handler.clone();
    let handler_2 = handler.clone();
    
    let id0 = handler.get_unique_id().await;
    let id1 = handler_1.get_unique_id().await;
    let id2 = handler_2.get_unique_id().await;

    println!("{} {} {}", id0, id1, id2);

}
