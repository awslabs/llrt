// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::uninlined_format_args)]

use std::{
    collections::VecDeque,
    env, fs,
    io::{stdout, IsTerminal},
    path::{Path, PathBuf},
};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::Print,
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
};

use crate::base::libs::{
    logging::format_values,
    utils::{error::ErrorExtensions, result::ResultExt},
};
// rquickjs components
use crate::base::{
    async_with, AsyncContext, CatchResultExt, Ctx, Error, EvalOptions, Object, PredefinedAtom,
    Promise, Rest,
};

use crate::VERSION_STRING;

async fn process_input(ctx: &Ctx<'_>, input: &str, tty: bool) -> String {
    // First try to evaluate and format the input

    let mut options: EvalOptions = EvalOptions::default();
    options.promise = true;

    match async {
        let promise = ctx.eval_with_options::<Promise, _>(input.as_bytes(), options)?;
        let future = promise.into_future::<Object>();
        let value = future.await?.get(PredefinedAtom::Value)?;
        format_values(ctx, Rest(vec![value]), tty, true)
    }
    .await
    .catch(ctx)
    {
        Ok(v) => v,
        Err(error) => {
            match (|| {
                let error_value = error.into_value(ctx)?;
                format_values(ctx, Rest(vec![error_value]), tty, true)
            })() {
                Ok(s) => s,
                Err(err) => err.to_string(),
            }
        },
    }
}

pub(crate) async fn run_repl(ctx: &AsyncContext) {
    let is_tty = stdout().is_terminal();
    async_with!(ctx => |ctx| {

        println!("Welcome to {}\nType \".exit\" or Ctrl+C or Ctrl+D to exit", VERSION_STRING);

        let history_file = if cfg!(windows) {
            env::var("APPDATA")
                .ok()
                .map(|path| PathBuf::from(path).join("llrtrepl"))
        } else {
            env::var("HOME")
                .ok()
                .map(|path| PathBuf::from(path).join(".config").join("llrtrepl"))
        };

        let mut persist_history = false;

        //initialize history
        let mut history: VecDeque<String> = if let Some(ref history_file) = history_file {

            if let Some(parent) = history_file.parent() {
                fs::create_dir_all(parent).or_throw(&ctx)?;
            }
            persist_history = true;
            if history_file.exists() {
                fs::read_to_string(history_file)?
                    .lines()
                    .map(String::from)
                    .collect()
            } else {
                VecDeque::new()
            }
        } else {
            VecDeque::new()
        };

        let mut current_input = String::new();
        let mut history_index = history.len();
        let mut cursor_pos = 0;

        println!("");

        let exit_repl = || {
            execute!(
                stdout(),
                cursor::MoveToColumn(0),
                Clear(ClearType::CurrentLine),
            )
        };

        enable_raw_mode()?;

        let mut added_input_chars = false;

        loop {
            execute!(
                stdout(),
                cursor::MoveToColumn(0),
                Clear(ClearType::CurrentLine),
                Print(format!("> {}", current_input)),
                cursor::MoveToColumn(cursor_pos as u16 + 2)
            )?;

            if let Event::Key(KeyEvent {
                code,
                modifiers,
                #[cfg(windows)]
                kind,
                ..
            }) = event::read()?
            {
                #[cfg(windows)]
                if kind == event::KeyEventKind::Release {
                  continue;
                }
                match code {
                    KeyCode::Enter => {
                        println!();
                        let cmd: String = current_input.trim().into();
                        if !cmd.is_empty() {
                            if cmd == ".exit" {
                                exit_repl()?;
                                break;
                            }
                            execute!(stdout(), cursor::MoveToColumn(0))?;
                            disable_raw_mode()?;
                            let output = process_input(&ctx, &cmd,is_tty).await;
                            println!("{output}");
                            enable_raw_mode()?;

                            //only push to history if we're not reusing the same command
                            if added_input_chars {
                                history.push_back(cmd);
                                if history.len() > 100 {
                                    history.pop_front();
                                }
                            }

                            history_index = history.len();
                            current_input.clear();
                            cursor_pos = 0;
                            if persist_history {
                                write_history(&history, history_file.as_deref());
                            }
                            added_input_chars = false;
                        }
                    },
                    KeyCode::Up => {
                        if !history.is_empty() && history_index > 0 {
                            added_input_chars = false;
                            history_index -= 1;
                            current_input = history[history_index].clone();
                            cursor_pos = current_input.len();
                        }
                    },
                    KeyCode::Down => {
                        let history_len =  history.len();
                        if  history_len > 0 {
                            match history_index.cmp(&(history_len - 1)) {
                                    std::cmp::Ordering::Less => {
                                        added_input_chars = false;
                                        history_index += 1;
                                        current_input = history[history_index].clone();
                                        cursor_pos = current_input.len();
                                    }
                                    std::cmp::Ordering::Equal => {
                                        added_input_chars = false;
                                        history_index = history_len;
                                        current_input.clear();
                                        cursor_pos = 0;
                                    }
                                    _ => {}
                                }
                        }
                    },
                    KeyCode::Left => {
                        cursor_pos = cursor_pos.saturating_sub(1);
                    },
                    KeyCode::Right => {
                        if cursor_pos < current_input.len() {
                            cursor_pos += 1;
                        }
                    },
                    KeyCode::Backspace => {
                        if cursor_pos > 0 {
                            current_input.remove(cursor_pos - 1);
                            cursor_pos -= 1;
                        }
                    },
                    KeyCode::Char(c) => {
                        if modifiers == KeyModifiers::CONTROL && (c == 'c' || c == 'd') {
                            exit_repl()?;
                            break;
                        }
                        added_input_chars = true;
                        current_input.insert(cursor_pos, c);
                        cursor_pos += 1;
                    },
                    _ => {},
                }
            }
        }

        disable_raw_mode()?;

        Ok::<_,Error>(())
    })
    .await
    .expect("Failed to run REPL")
}

fn write_history(history: &VecDeque<String>, history_file: Option<&Path>) {
    if let Some(history_file) = history_file {
        let _ = fs::write(
            history_file,
            history
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
        );
    }
}

#[cfg(test)]
mod tests {

    use llrt_core::libs::utils::primordials::{BasePrimordials, Primordial};
    use llrt_test::test_async_with;

    use crate::repl::process_input;

    #[tokio::test]
    async fn test_process_input() {
        test_async_with(|ctx| {
            Box::pin(async move {
                BasePrimordials::init(&ctx).unwrap();
                let output = process_input(&ctx, "throw new Error('err')", false).await;

                assert_eq!(output, "Error: err\n  at <eval> (eval_script:1:10)");

                let output = process_input(&ctx, "Promise.reject(1)", false).await;

                assert_eq!(output, "Promise {\n  <rejected> 1\n}");

                let output = process_input(&ctx, "1+1", false).await;

                assert_eq!(output, "2");

                let output = process_input(&ctx, "a", false).await;

                assert_eq!(
                    output,
                    "ReferenceError: a is not defined\n  at <eval> (eval_script:1:1)"
                );
            })
        })
        .await;
    }
}
