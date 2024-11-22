use llrt_modules::VERSION;
use llrt_utils::error::ErrorExtensions;
use rquickjs::{prelude::Rest, AsyncContext, CatchResultExt, Ctx, Result, Value};

use crate::modules::console::{self};

fn process_input(ctx: &Ctx<'_>, input: &str) -> String {
    match ctx
        .eval::<Value, _>(input.as_bytes())
        .map(|v| console::format(ctx, Rest(vec![v])).expect("Failed to format"))
        .catch(ctx)
    {
        Ok(s) => s,
        Err(caught_err) => {
            match caught_err
                .into_value(ctx)
                .map(|v| console::format(ctx, Rest(vec![v])).expect("Failed to format"))
            {
                Ok(s) => s,
                Err(caught_err) => format!("Error: {:?}", caught_err),
            }
        },
    }
}

pub(crate) async fn run_repl(ctx: &AsyncContext) {
    ctx.with(|ctx| -> Result<()> {
        let mut input = String::new();
        println!("Welcome to llrt v{}", VERSION);
        loop {
            print!("> ");
            std::io::Write::flush(&mut std::io::stdout())?;
            std::io::stdin().read_line(&mut input)?;
            println!("{}", process_input(&ctx, &input));
            input.clear();
        }
    })
    .await
    .expect("Failed to run REPL")
}

#[cfg(test)]
mod tests {
    use crate::modules::repl::process_input;
    use llrt_test::test_sync_with;
    use std::io::{stdout, IsTerminal};

    #[tokio::test]
    async fn test_process_input() {
        test_sync_with(|ctx| {
            let output = process_input(&ctx, "1+1");
            let is_tty = stdout().is_terminal();
            let expect = if is_tty { "\u{1b}[33m2\u{1b}[0m" } else { "2" };
            assert_eq!(output, expect);

            let output = process_input(&ctx, "a");
            let expect = if is_tty {
                "ReferenceError: a is not defined\u{1b}[30m\n  at <eval> (eval_script:1:1)\u{1b}[0m"
            } else {
                "ReferenceError: a is not defined\n  at <eval> (eval_script:1:1)"
            };
            assert_eq!(output, expect);
            Ok(())
        })
        .await;
    }
}
