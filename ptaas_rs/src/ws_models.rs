use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
enum WSMessage {
    FromServer(WSFromServer),
    FromClient(WSFromClient),
}

#[derive(Serialize, Deserialize, Debug)]
enum WSFromServer {}

#[derive(Serialize, Deserialize, Debug)]
enum WSFromClient {
    Subscribe(SubscribeMessage),
    Unsubscribe(UnsubscribeMessage),
}

#[derive(Serialize, Deserialize, Debug)]
struct SubscribeMessage {
    project_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct UnsubscribeMessage {
    project_id: String,
}

fn dummy_msgs() {
    // create a dummy WSFromClient messages
    let subscribe_msg = WSMessage::FromClient(WSFromClient::Subscribe(SubscribeMessage {
        project_id: String::from("project1"),
    }));

    let unsubscribe_msg = WSMessage::FromClient(WSFromClient::Unsubscribe(UnsubscribeMessage {
        project_id: String::from("project1"),
    }));

    // now print them out with serde_json
    println!(
        "Subscribe Message:\n{}\n",
        serde_json::to_string(&subscribe_msg).unwrap()
    );

    println!(
        "Unsubscribe Message:\n{}\n",
        serde_json::to_string(&unsubscribe_msg).unwrap()
    );
}
