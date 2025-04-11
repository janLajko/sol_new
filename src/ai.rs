// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use egg_mode::search::{self, ResultType};

use std::io::{stdin, BufRead, Write}; 

#[tokio::test]
async fn test_x() {
    dotenv::dotenv().ok();
    let config = load().await.unwrap();

    println!("Search term:");
    let line = stdin().lock().lines().next().unwrap().unwrap();

    let search = search::search(line)
        .result_type(ResultType::Recent)
        .count(10) 
        .call(&config.token)
        .await
        .unwrap();

    for tweet in &search.statuses {
        println!("{}", tweet.text);
    }
}


pub struct Config {
    pub token: egg_mode::Token,
    pub user_id: u64,
    pub screen_name: String,
}

/// This needs to be a separate function so we can retry after creating the
/// twitter_settings file. Idealy we would recurse, but that requires boxing
/// the output which doesn't seem worthwhile
async fn load() -> Option<Config> {
    //IMPORTANT: make an app for yourself at apps.twitter.com and get your
    //key/secret into these files; these examples won't work without them
    let consumer_key = std::env::var("CK").expect("consumer_key not found");
    let consumer_secret = std::env::var("CS").expect("consumer_secret not found");

    let con_token = egg_mode::KeyPair::new(consumer_key, consumer_secret);

    let mut config = String::new();
    let user_id: u64;
    let username: String;
    let token: egg_mode::Token;

    //look at all this unwrapping! who told you it was my birthday?
    
    let request_token = egg_mode::auth::request_token(&con_token, "oob")
        .await
        .unwrap();

    println!("Go to the following URL, sign in, and give me the PIN that comes back:");
    println!("{}", egg_mode::auth::authorize_url(&request_token));

    let mut pin = String::new();
    std::io::stdin().read_line(&mut pin).unwrap();
    println!("");

    let tok_result = egg_mode::auth::access_token(con_token, &request_token, pin)
        .await
        .unwrap();

    token = tok_result.0;
    user_id = tok_result.1;
    username = tok_result.2;

    match token {
        egg_mode::Token::Access {
                access: ref access_token,
                ..
            } => {
                config.push_str(&username);
                config.push('\n');
                config.push_str(&format!("{}", user_id));
                config.push('\n');
                config.push_str(&access_token.key);
                config.push('\n');
                config.push_str(&access_token.secret);
            }
            _ => unreachable!(),
    }

    let mut f = std::fs::File::create("twitter_settings").unwrap();
    f.write_all(config.as_bytes()).unwrap();

    println!("Welcome, {}, let's get this show on the road!", username);
    

    //TODO: Is there a better way to query whether a file exists?
    if std::fs::metadata("twitter_settings").is_ok() {
        Some(Config {
            token: token,
            user_id: user_id,
            screen_name: username, 
        })
    } else {
        None
    }
}
