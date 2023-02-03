use crate::file_handling::{execute_instruction, is_inside_root};
use crate::file_handling::{DELETE_INSTR, GET_INSTR, POST_INSTR, PUT_INSTR};

use std::io;

use std::process;
use std::str;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[derive(PartialEq)]
pub enum ParseError {
    WrongMethod,
    UnkownMethod,
    NotInRoot,
    UnknownInstruction,
}

#[derive(PartialEq)]
pub enum RequestVerbs {
    GET,
    POST,
    DELETE,
    PUT,
    UNKOWN,
}

macro_rules! debug_print {
    ($ex: expr) => {
        if cfg!(debug_assertions) {
            println!("{}", $ex)
        }
    };
}

pub async fn send_response(socket: &mut tokio::net::TcpStream, response: String) {
    match socket.write_all(response.as_bytes()).await {
        Ok(_) => {}
        Err(e) => {
            debug_print!(format!("Error while sending to socket: {}", e));
            socket.shutdown().await.unwrap();
            process::exit(1);
        }
    }
    match socket.flush().await {
        Ok(_) => {}
        Err(e) => {
            debug_print!(format!("Error while flushing socket: {}", e));
            socket.shutdown().await.unwrap();
            process::exit(1);
        }
    }
}

pub fn ok_200(message: impl Into<String>) -> String {
    let msg = message.into();
    format!("HTTP/1.1 200 OK\r\n\r\n{msg}")
}

pub fn bad_400(message: impl Into<String>) -> String {
    let msg = message.into();
    format!("HTTP/1.1 400 Bad\r\n\r\n{msg}")
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

///Return (Instruction, Path, Option<Content>, Method)
pub async fn parse_json(json_like: &str) -> Option<(String, String, Option<String>, String)> {
    let json_iter = json_like.chars();

    let mut method = String::with_capacity(7);

    for c in json_iter.clone() {
        if c == '/' {
            break;
        }

        method.push(c);
    }

    method = method.trim().into();

    let json_start = json_like.find("{");
    let json_end = json_like.find("}");

    if json_start.is_none() || json_end.is_none() {
        return None;
    }

    let mut json_as_string = String::new();

    for i in json_start.unwrap() + 1..json_end.unwrap() {
        json_as_string.push(json_iter.clone().nth(i).unwrap());
    }

    json_as_string = json_as_string.replace(&['"'][..], "");

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

    let instruction: String = split_json[1][1].trim().into();
    let path: String = split_json[2][1].trim().into();
    let mut content: Option<String> = Some(split_json[0][1].trim().into());

    if split_json[0][1].len() == 0 {
        content = None;
    }

    return Some((instruction, path, content, method));
}

fn instr_to_verb(instr: &String) -> RequestVerbs {
    let instr_into = &instr.as_str();

    if POST_INSTR.contains(instr_into) {
        return RequestVerbs::POST;
    }

    if DELETE_INSTR.contains(instr_into) {
        return RequestVerbs::DELETE;
    }

    if GET_INSTR.contains(instr_into) {
        return RequestVerbs::GET;
    }

    if PUT_INSTR.contains(instr_into) {
        return RequestVerbs::PUT;
    }

    return RequestVerbs::UNKOWN;
}

fn string_method_to_enum(method: &String) -> RequestVerbs {
    let cleaned_method = method.trim().to_lowercase();

    match &*cleaned_method {
        "post" => RequestVerbs::POST,
        "delete" => RequestVerbs::DELETE,
        "get" => RequestVerbs::GET,
        "put" => RequestVerbs::PUT,
        _ => RequestVerbs::UNKOWN,
    }
}

pub async fn parse_params(
    instr: &String,
    path: &String,
    text: &Option<String>,
    method: &String,
    socket: &mut tokio::net::TcpStream,
) -> Result<(), ParseError> {
    if !is_inside_root(&path) {
        return Err(ParseError::NotInRoot);
    }

    match instr_to_verb(instr) {
        RequestVerbs::UNKOWN => {
            return Err(ParseError::UnknownInstruction);
        }
        verbs => match string_method_to_enum(method) {
            RequestVerbs::UNKOWN => return Err(ParseError::WrongMethod),
            known_verbs => {
                if known_verbs != verbs {
                    return Err(ParseError::WrongMethod);
                }
            }
        },
    }

    execute_instruction(&instr, &path, &text, socket).await;

    Ok(())
}

#[tokio::main]
pub async fn start_server() -> io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;

    loop {
        match listener.accept().await {
            Ok((mut socket, _)) => {
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
                        Some((instr, path, text, method)) => {
                            debug_print!(format!(
                                "Instr: {:?}\nPath: {:?}\nText: {:?}\nMethod: {:?}\n",
                                instr, path, text, method
                            ));
                            match parse_params(&instr, &path, &text, &method, &mut socket).await {
                                Ok(_) => {}
                                Err(e) => {
                                    let response = match e {
                                        ParseError::NotInRoot => {
                                            bad_400(&*format!("Supplied path: `{path}` is not inside ./root"))
                                        }
                                        ParseError::WrongMethod => {bad_400(&*format!("Supplied method: `{method}` is not supported for given instruction: `{instr}`"))}
                                        ParseError::UnkownMethod => {
                                            bad_400(&*format!("Supplied method: `{method}` is unknown or not supported"))
                                        }
                                        ParseError::UnknownInstruction =>{
                                            bad_400(&*format!("Supplied instr: `{instr}` is unknown or not supported"))
                                        }
                                    };

                                    send_response(&mut socket, response).await;
                                }
                            }
                        }
                        None => send_response(&mut socket, bad_400("No payload provided")).await,
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
