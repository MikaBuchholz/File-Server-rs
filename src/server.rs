use crate::file_handling::build_json_response;
use crate::file_handling::{execute_instruction, is_inside_root};

use std::collections::HashMap;
use std::io;

use std::str;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub async fn send_response(socket: &mut tokio::net::TcpStream, response: String) -> io::Result<()> {
    socket.write_all(response.as_bytes()).await?;
    Ok(())
}

pub async fn extract_json(json_like: &str) -> Option<(String, String)> {
    let json_iter = json_like.chars();
    let ln = json_like.find("Content-Length");

    if ln.is_none() {
        return None;
    }

    let mut eof_content: usize = 4;

    let mut content_len_str = String::new();
    for index in ln.unwrap() + 14..json_like.len() {
        let cur_char = json_iter.clone().nth(index).unwrap().to_string();

        if cur_char == "\r" {
            eof_content += index;
            break;
        }

        let parsed_char = cur_char.parse::<usize>();

        if parsed_char.is_ok() {
            content_len_str.push(cur_char.chars().next().unwrap());
        }
    }

    let content_length = content_len_str.parse::<usize>().unwrap();

    if content_length == 0 {
        return None;
    }

    let json_str = &json_like[eof_content..eof_content + content_length]
        .replace(&['\n', '\t', '}', '{', '"'][..], "");

    let json: Vec<_> = json_str
        .split(",")
        .collect::<Vec<&str>>()
        .iter()
        .map(|el| el.split(":").collect::<Vec<&str>>())
        .collect();

    Some((json[0][1].trim().to_string(), json[1][1].trim().to_string()))
}

pub async fn parse_params(
    instr: &String,
    path: &String,
    socket: &mut tokio::net::TcpStream,
    cache: &mut HashMap<(String, String), (String, usize)>,
) -> Option<()> {
    if !is_inside_root(&path) {
        return None;
    }

    let _ = execute_instruction(&instr, &path, socket, cache).await;

    Some(())
}

#[tokio::main]
pub async fn start_server() -> io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;

    let mut buffer = [0; 256];

    let mut cache: HashMap<(String, String), (String, usize)> = HashMap::new();

    loop {
        let (mut socket, _) = listener.accept().await?;

        socket.read(&mut buffer[..]).await?;

        let payload = str::from_utf8(&buffer).unwrap().replace("\0", "");
        //TODO: invalidate/update cache after write operations (TODO: write ops) to specific cache location
        match extract_json(&payload).await {
            Some((instr, path)) => match cache.get(&(instr.clone(), path.clone())) {
                Some((content, len)) => {
                    println!("Cache hit!");
                    send_response(
                        &mut socket,
                        build_json_response((*content.clone()).to_string(), *len),
                    )
                    .await?
                }
                None => match parse_params(&instr, &path, &mut socket, &mut cache).await {
                    Some(_) => {
                        println!("Cache miss");
                        println!(
                            "Instruction: `{}` on path: `{}` executed succesfuly",
                            instr, path
                        )
                    }
                    None => {
                        send_response(
                            &mut socket,
                            "HTTP/1.1 400 Bad\r\n\r\nProvided path is not authorized!".into(),
                        )
                        .await?
                    }
                },
            },
            None => {
                send_response(
                    &mut socket,
                    "HTTP/1.1 400 Bad\r\n\r\nNo payload provided\r\n\r\n".into(),
                )
                .await?;
            }
        }

        socket.flush().await?;
    }
}
