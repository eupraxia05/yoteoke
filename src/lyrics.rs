pub struct ParsedLyrics {
    pub blocks: Vec<Block>,
}

impl ParsedLyrics {
    pub fn parse(lyrics: &String) -> Result<ParsedLyrics, String> {        
        let mut blocks = Vec::new();
        let lines = lyrics.lines();

        let mut curr_block = Block::default();
        for line in lines {
            if !line.is_empty() {
                curr_block.lines.push(line.trim().into())
            } else {
                if curr_block.lines.len() > 0 {
                    blocks.push(curr_block.clone());
                    curr_block = Block::default();
                }
            }
        }
        
        Ok(ParsedLyrics {
            blocks,
        })
    }
}

#[derive(Default, Clone)]
pub struct Block {
    pub lines: Vec<String>,
}