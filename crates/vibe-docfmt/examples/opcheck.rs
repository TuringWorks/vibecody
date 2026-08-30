//! Does a page's content survive lopdf's decode → encode → decode?
use lopdf::content::Content;
use lopdf::Document;
fn main() {
    let path = std::env::args().nth(1).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let doc = Document::load_mem(&bytes).unwrap();
    for (n, id) in doc.get_pages() {
        let Ok(content) = doc.get_page_content(id) else { continue };
        let ops = Content::decode(&content).unwrap().operations;
        let re = Content { operations: ops.clone() }.encode().unwrap();
        match Content::decode(&re) {
            Ok(back) => {
                if back.operations.len() != ops.len() {
                    println!("page {n}: {} ops -> {} ops", ops.len(), back.operations.len());
                    for (i, (a, b)) in ops.iter().zip(back.operations.iter()).enumerate() {
                        if a.operator != b.operator {
                            println!("  first divergence at {i}: {:?} vs {:?}", a.operator, b.operator);
                            for k in i.saturating_sub(3)..(i + 2).min(ops.len()) {
                                println!("    old[{k}] {} {:?}", ops[k].operator, &format!("{:?}", ops[k].operands)[..120.min(format!("{:?}", ops[k].operands).len())]);
                            }
                            break;
                        }
                    }
                }
            }
            Err(e) => println!("page {n}: re-decode failed: {e}"),
        }
        if n > 3 { break; }
    }
}
