use std::path::PathBuf;

fn main() {
    let out_dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("out/analysis"));
    macroquad::Window::from_config(
        macroquad::conf::Conf {
            miniquad_conf: macroquad::miniquad::conf::Conf {
                window_title: "c64re hybrid shell".to_string(),
                window_width: 640,
                window_height: 400,
                ..Default::default()
            },
            ..Default::default()
        },
        run(out_dir),
    );
}

async fn run(out_dir: PathBuf) {
    match c64re_hybrid::load_session(&out_dir) {
        Ok(session) => {
            println!(
                "playing {} samples from {} (game start frame {:?})",
                session.samples.len(),
                out_dir.display(),
                session.game_start_frame
            );
            println!(
                "WASD/arrows = joystick, SPACE = fire, TAB = autoplay, R = restart, ESC = quit"
            );
            c64re_hybrid::run_player(&session).await;
        }
        Err(err) => {
            eprintln!("error: {err}");
            std::process::exit(1);
        }
    }
}
