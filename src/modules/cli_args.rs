use clap::Parser;

/// An TUI application to view and analyze VCD files
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct CliArgs {
    /// The path to the VCD file to open
    #[arg(
        short = 'f',
        long = "file",
        default_value = "./assets/verilog/test_1.vcd"
    )]
    pub file_path: String,
}
