use crate::file_handling::build_json_response;
use crate::file_handling::{execute_instruction, is_inside_root};

use std::collections::HashMap;
use std::io;

use std::str;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

macro_rules! debug_print {
    ($ex: expr) => {
        if cfg!(debug_assertions) {
            println!("{}", $ex)
        }
    };
}

pub async fn send_response(socket: &mut tokio::net::TcpStream, response: String) -> io::Result<()> {
    socket.write_all(response.as_bytes()).await?;
    socket.flush().await?;
    Ok(())
}

pub fn ok_200(message: &str) -> String {
    format!("HTTP/1.1 200 OK\r\n\r\n{message}")
}

pub fn bad_400(message: &str) -> String {
    format!("HTTP/1.1 400 Bad\r\n\r\n{message}")
}

fn json_format_is_valid(json: &Vec<Vec<&str>>) -> bool {
    if json.len() != 3 {
        return false;
    }

    for sub_vec in json {
        if sub_vec.len() != 2 {
            return false;
        }
    }

    return true;
}

///Return (Instruction, Path, Option<Content>)
pub async fn parse_json(json_like: &str) -> Option<(String, String, Option<String>)> {
    let json_iter = json_like.chars();

    let json_start = json_like.find("{");
    let json_end = json_like.find("}");

    if json_start.is_none() || json_end.is_none() {
        return None;
    }

    let mut json_as_string = String::new();

    for i in json_start.unwrap() + 1..json_end.unwrap() {
        json_as_string.push(json_iter.clone().nth(i).unwrap());
    }

    json_as_string = json_as_string.replace(&['\n', '\t', '"'][..], "");

    let mut split_json: Vec<Vec<&str>> = json_as_string
        .split(",")
        .map(|elem| elem.split(":").collect())
        .collect();

    if split_json.len() == 2 {
        //This ensures that the format bellow will be true
        split_json.push(vec!["", ""])
    }

    if !json_format_is_valid(&split_json) {
        return None;
    }

    //This ensures that if provided following pattern will be true:
    //[[content, ...], [instr, ...], [path, ...]]

    split_json.sort_by(|a, b| a[0].cmp(b[0]));

    if split_json[0][1].len() == 0 {
        return Some((
            split_json[1][1].trim().into(), // Instruction
            split_json[2][1].trim().into(), // Path
            None,                           // Text
        ));
    } else {
        return Some((
            split_json[1][1].trim().into(),       // Instruction
            split_json[2][1].trim().into(),       // Path
            Some(split_json[0][1].trim().into()), //Text
        ));
    }
}

pub async fn parse_params(
    instr: &String,
    path: &String,
    text: &Option<String>,
    socket: &mut tokio::net::TcpStream,
    cache: &mut HashMap<(String, String), (String, usize)>,
) -> Option<()> {
    if !is_inside_root(&path) {
        return None;
    }

    let _ = execute_instruction(&instr, &path, &text, socket, cache).await;

    Some(())
}

#[tokio::main]
pub async fn start_server() -> io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;

    let cache: HashMap<(String, String), (String, usize)> = HashMap::new();

    loop {
        match listener.accept().await {
            Ok((mut socket, _)) => {
                let mut cache = cache.clone();

                debug_print!(format!("Request: {:?}", socket.peer_addr()));

                tokio::spawn(async move {
                    let mut buffer = [0; 256];

                    match socket.read(&mut buffer[..]).await {
                        Ok(_) => {}
                        Err(e) => {
                            debug_print!(format!("Error while reading from socket: {}", e));
                            socket.shutdown().await.unwrap();
                            return;
                        }
                    }

                    let payload = str::from_utf8(&buffer).unwrap().replace("\0", "");

                    match parse_json(&payload).await {
                        Some((instr, path, text)) => {
                            match cache.get(&(instr.clone(), path.clone())) {
                                Some((content, len)) => {
                                    match send_response(
                                        &mut socket,
                                        build_json_response((*content.clone()).to_string(), *len),
                                    )
                                    .await
                                    {
                                        Ok(_) => {}
                                        Err(e) => {
                                            debug_print!(format!(
                                                "Error while sending to socket: {}",
                                                e
                                            ));
                                            socket.shutdown().await.unwrap();
                                            return;
                                        }
                                    };
                                }
                                None => {
                                    parse_params(&instr, &path, &text, &mut socket, &mut cache)
                                        .await;
                                }
                            };
                        }
                        None => {
                            match send_response(&mut socket, bad_400("No payload provided")).await {
                                Ok(_) => {}
                                Err(e) => {
                                    debug_print!(format!("Error while sending to socket: {}", e));
                                    socket.shutdown().await.unwrap();
                                    return;
                                }
                            }
                        }
                    }
                });
            }
            Err(e) => {
                debug_print!(format!("Error accepting connection: {}", e));
                break;
            }
        }
    }
    Ok(())
}
