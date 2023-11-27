use std::{env, fs, io::Result};

use evaluator::program::eval_program;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Usage: {} <filename>", args[0]);

        return Ok(());
    }

	let abs_path = fs::canonicalize(&args[1]);

	if let Err(error) = abs_path {
		return Err(error);
	}

	let ok_abs_path = abs_path.unwrap();

	let file_name = ok_abs_path.to_str().unwrap();

    let content = fs::read_to_string(file_name)?;

    eval_program(file_name, &content)?;

    Ok(())
}
