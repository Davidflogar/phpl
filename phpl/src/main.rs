use std::{
    env, fs,
    io::Result,
};

use evaluator::program::eval_program;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Usage: {} <filename>", args[0]);

        return Ok(());
    }

    let file_name = &args[1];

    let content = fs::read_to_string(file_name)?;

    eval_program(&file_name, &content)?;

    Ok(())
}
