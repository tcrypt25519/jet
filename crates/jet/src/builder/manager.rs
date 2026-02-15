use log::info;
use syntect::{
    easy::HighlightLines,
    highlighting::{Color, ThemeSet},
    parsing::SyntaxSet,
    util::{LinesWithEndings, as_24_bit_terminal_escaped},
};

use jet_runtime::{Address, exec};

use crate::builder::{Error, contract, env::Env};

pub struct Manager<'ctx> {
    build_env: Env<'ctx>,
}

impl<'ctx> Manager<'ctx> {
    pub fn new(build_env: Env<'ctx>) -> Self {
        Self { build_env }
    }

    pub fn env(&self) -> &Env<'ctx> {
        &self.build_env
    }

    pub fn add_contract_function(&self, addr: Address, rom: &[u8]) -> Result<(), Error> {
        let fn_name = exec::mangle_contract_fn(&addr);
        info!("Building ROM into function {}", fn_name);

        contract::build(&self.build_env, &fn_name, rom)?;

        if self.build_env.opts().emit_llvm() {
            self.print_ir();
        }

        if self.build_env.opts().assert() {
            if !self.verify_contract(addr) {
                return Err(Error::Verify);
            }
            self.build_env.module().verify()?;
        }
        Ok(())
    }

    fn verify_contract(&self, addr: Address) -> bool {
        let func_name = exec::mangle_contract_fn(&addr);
        self.build_env
            .module()
            .get_function(&func_name)
            .map(|func| func.verify(true))
            .unwrap_or(false)
    }

    fn print_ir(&self) {
        let ts = ThemeSet::load_defaults();
        let ps = match SyntaxSet::load_from_folder("contrib/syntaxes") {
            Ok(ps) => ps,
            Err(e) => {
                eprintln!("Warning: Failed to load syntax set: {}", e);
                return;
            }
        };
        let syntax = match ps.find_syntax_by_extension("ll") {
            Some(syntax) => syntax,
            None => {
                eprintln!("Warning: Failed to find LLVM syntax");
                return;
            }
        };

        let mut theme = ts.themes["base16-eighties.dark"].clone();
        theme.settings.background = Some(Color {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        });

        let mut h = HighlightLines::new(syntax, &theme);

        let s = self.build_env.module().print_to_string().to_string();

        println!();
        for line in LinesWithEndings::from(s.as_str()) {
            match h.highlight_line(line, &ps) {
                Ok(ranges) => {
                    let escaped = as_24_bit_terminal_escaped(&ranges[..], true);
                    print!("    {}", escaped);
                }
                Err(e) => {
                    eprintln!("Warning: Failed to highlight line: {}", e);
                    print!("    {}", line);
                }
            }
        }
        println!();
    }
}
