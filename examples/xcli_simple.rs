use log::{debug, info, LevelFilter};
use xcli::*;

fn main() {
    env_logger::init();

    info!("cli started");

    let mut app = App::new("xCLI")
        .version("v0.1")
        .author("kingwel.xie@139.com");

    app.add_subcommand(Command::new("qwert")
        .about("controls testing features")
        .action(|_app, _| -> XcliResult {
            println!("tested");
            log::set_max_level(LevelFilter::Info);
            Ok(CmdExeCode::Ok)
        }));

    app.run();
}