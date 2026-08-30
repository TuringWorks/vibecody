use std::path::PathBuf;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let src = PathBuf::from(&a[1]);
    let from: usize = a[2].parse().unwrap();
    let to: usize = a[3].parse().unwrap();
    let buf = vibe_docfmt::read_text(&src).unwrap();
    let re = vibe_docfmt::render(&vibe_docfmt::parse_text(buf.format, &buf.text));
    let w: Vec<&str> = buf.text.lines().collect();
    let g: Vec<&str> = re.lines().collect();
    for i in from..to.min(w.len()) {
        println!("{i:5} W {:?}", &w[i][..w[i].len().min(120)]);
    }
    println!("---");
    for i in from..to.min(g.len()) {
        println!("{i:5} G {:?}", &g[i][..g[i].len().min(120)]);
    }
}
