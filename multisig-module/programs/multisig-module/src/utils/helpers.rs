use anchor_lang::prelude::*;

pub fn calculate_instruction_complexity(data: &[u8]) -> Result<u32> {
    let mut complexity = 0u32;
    
    // Simple heuristic: 1 point per 10 bytes + bonus for certain patterns
    complexity += (data.len() as u32) / 10;
    
    // Add complexity for potential loops/calls (simplified)
    for chunk in data.chunks(4) {
        if chunk.len() == 4 {
            let value = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            if value > 1000000 { // Large numbers might indicate loops
                complexity += 10;
            }
        }
    }
    
    Ok(complexity)
}