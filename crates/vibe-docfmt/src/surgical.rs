//! The edit engine shared by the XML formats.
//!
//! Reading gives a list of *slots*: a path into the XML tree plus the [`Block`]
//! that path represents. Saving aligns the old slot list against the blocks
//! parsed from the edited buffer and applies the smallest set of element-level
//! operations that gets from one to the other — rewrite, insert, delete. Every
//! element not named by an operation is left exactly as it was found.

use similar::{Algorithm, DiffOp, TextDiff};

use crate::error::DocError;
use crate::markdown;
use crate::model::{Block, Warning};
use crate::xmltree::{Element, Node};

/// A block found in the tree, and where it lives.
#[derive(Debug, Clone)]
pub struct Slot {
    /// Child indices from the container element down to the block's element.
    pub path: Vec<usize>,
    pub block: Block,
}

/// What a format needs to provide for the engine to edit it.
pub trait BlockAdapter {
    /// Rewrite an existing element to carry `block`, preserving everything the
    /// block model does not describe (images, footnote refs, comments …).
    fn rewrite(&mut self, el: &mut Element, block: &Block) -> Result<Vec<Warning>, DocError>;

    /// Build a new element for `block`, using `template` (the element of the
    /// nearest preceding slot, when there is one) for style inheritance.
    fn build(&mut self, template: Option<&Element>, block: &Block) -> Result<Element, DocError>;

    /// Whether a block of this kind may be inserted at `path`'s level.
    /// Returning an error rejects the whole write with an explanation rather
    /// than writing something the format cannot express.
    fn check_insert(&self, block: &Block, template: Option<&Element>) -> Result<(), DocError>;
}

/// Look up an element by path.
pub fn get_mut<'a>(container: &'a mut Element, path: &[usize]) -> Option<&'a mut Element> {
    let mut current = container;
    for index in path {
        let node = current.children.get_mut(*index)?;
        current = match node {
            Node::Element(e) => e,
            _ => return None,
        };
    }
    Some(current)
}

/// Look up an element by path, immutably.
pub fn get<'a>(container: &'a Element, path: &[usize]) -> Option<&'a Element> {
    let mut current = container;
    for index in path {
        let node = current.children.get(*index)?;
        current = match node {
            Node::Element(e) => e,
            _ => return None,
        };
    }
    Some(current)
}

fn parent_of<'a>(container: &'a mut Element, path: &[usize]) -> Option<(&'a mut Element, usize)> {
    let (last, head) = path.split_last()?;
    let parent = get_mut(container, head)?;
    Some((parent, *last))
}

/// Apply the block list from an edited buffer to the tree.
pub fn apply<A: BlockAdapter>(
    container: &mut Element,
    slots: &[Slot],
    new_blocks: &[Block],
    adapter: &mut A,
) -> Result<Vec<Warning>, DocError> {
    let old_keys: Vec<String> = slots.iter().map(|s| block_key(&s.block)).collect();
    let new_keys: Vec<String> = new_blocks.iter().map(block_key).collect();

    let old_refs: Vec<&str> = old_keys.iter().map(String::as_str).collect();
    let new_refs: Vec<&str> = new_keys.iter().map(String::as_str).collect();
    let diff = TextDiff::configure()
        .algorithm(Algorithm::Myers)
        .diff_slices(&old_refs, &new_refs);

    // Reverse order keeps every path in `slots` valid while we mutate: an edit
    // late in the document cannot shift the indices of an earlier one.
    let ops: Vec<DiffOp> = diff.ops().to_vec();
    let mut warnings = Vec::new();
    for op in ops.iter().rev() {
        match *op {
            DiffOp::Equal { .. } => {}
            DiffOp::Replace {
                old_index,
                old_len,
                new_index,
                new_len,
            } => {
                let overlap = old_len.min(new_len);
                for k in 0..overlap {
                    let slot = &slots[old_index + k];
                    let block = &new_blocks[new_index + k];
                    let el = get_mut(container, &slot.path).ok_or_else(|| {
                        DocError::Structure(format!("lost track of element at {:?}", slot.path))
                    })?;
                    warnings.extend(adapter.rewrite(el, block)?);
                }
                // Longer on one side: the tail is a plain insert or delete.
                if new_len > old_len {
                    // `old_len` is at least 1 for a Replace, but an anchor read
                    // from an empty range would index out of bounds rather than
                    // fail cleanly, so it is derived defensively.
                    let anchor = old_index
                        .checked_add(old_len)
                        .and_then(|end| end.checked_sub(1))
                        .and_then(|i| slots.get(i))
                        .ok_or_else(|| {
                            DocError::Structure("replacement has no anchor element".into())
                        })?;
                    insert_after(
                        container,
                        &anchor.path,
                        &new_blocks[new_index + overlap..new_index + new_len],
                        adapter,
                        &mut warnings,
                    )?;
                } else if old_len > new_len {
                    for k in (new_len..old_len).rev() {
                        remove_at(container, &slots[old_index + k].path)?;
                    }
                }
            }
            DiffOp::Delete {
                old_index, old_len, ..
            } => {
                for k in (0..old_len).rev() {
                    remove_at(container, &slots[old_index + k].path)?;
                }
            }
            DiffOp::Insert {
                old_index,
                new_index,
                new_len,
            } => {
                let blocks = &new_blocks[new_index..new_index + new_len];
                match old_index.checked_sub(1).and_then(|i| slots.get(i)) {
                    Some(anchor) => {
                        insert_after(container, &anchor.path, blocks, adapter, &mut warnings)?
                    }
                    None => match slots.first() {
                        // Insert before the first known block.
                        Some(first) => {
                            insert_before(container, &first.path, blocks, adapter, &mut warnings)?
                        }
                        // A document with no blocks at all: append to the container.
                        None => {
                            let mut template: Option<Element> = None;
                            for block in blocks {
                                adapter.check_insert(block, template.as_ref())?;
                                let el = adapter.build(template.as_ref(), block)?;
                                container.children.push(Node::Element(el.clone()));
                                template = Some(el);
                            }
                        }
                    },
                }
            }
        }
    }
    Ok(warnings)
}

fn insert_after<A: BlockAdapter>(
    container: &mut Element,
    anchor_path: &[usize],
    blocks: &[Block],
    adapter: &mut A,
    warnings: &mut [Warning],
) -> Result<(), DocError> {
    insert_at_offset(container, anchor_path, 1, blocks, adapter, warnings)
}

fn insert_before<A: BlockAdapter>(
    container: &mut Element,
    anchor_path: &[usize],
    blocks: &[Block],
    adapter: &mut A,
    warnings: &mut [Warning],
) -> Result<(), DocError> {
    insert_at_offset(container, anchor_path, 0, blocks, adapter, warnings)
}

fn insert_at_offset<A: BlockAdapter>(
    container: &mut Element,
    anchor_path: &[usize],
    offset: usize,
    blocks: &[Block],
    adapter: &mut A,
    _warnings: &mut [Warning],
) -> Result<(), DocError> {
    let template = get(container, anchor_path).cloned();
    for block in blocks {
        adapter.check_insert(block, template.as_ref())?;
    }
    let built: Vec<Element> = blocks
        .iter()
        .map(|block| adapter.build(template.as_ref(), block))
        .collect::<Result<_, _>>()?;

    let (parent, index) = parent_of(container, anchor_path)
        .ok_or_else(|| DocError::Structure(format!("no parent for path {anchor_path:?}")))?;
    let at = (index + offset).min(parent.children.len());
    for (k, el) in built.into_iter().enumerate() {
        parent.children.insert(at + k, Node::Element(el));
    }
    Ok(())
}

fn remove_at(container: &mut Element, path: &[usize]) -> Result<(), DocError> {
    let (parent, index) = parent_of(container, path)
        .ok_or_else(|| DocError::Structure(format!("no parent for path {path:?}")))?;
    if index >= parent.children.len() {
        return Err(DocError::Structure(format!("stale path {path:?}")));
    }
    parent.children.remove(index);
    Ok(())
}

/// The comparison key for alignment: a block's full Markdown, so a change of
/// emphasis counts as a change even when the plain text is identical.
///
/// Only the separator newlines are trimmed. Trimming whitespace as well made a
/// paragraph that had lost a trailing space — Word ends them with a non-breaking
/// one constantly — compare equal to the one on disk, so the element was left
/// alone and the save then failed to verify against text the file did not carry.
pub fn block_key(block: &Block) -> String {
    let doc = crate::model::Document {
        format: crate::model::DocFormat::Docx,
        sections: vec![crate::model::Section {
            id: String::new(),
            title: None,
            blocks: vec![block.clone()],
        }],
        warnings: Vec::new(),
    };
    markdown::to_markdown(&doc)
        .trim_end_matches('\n')
        .to_string()
}
