use crate::pipeline::stt::Segment;

/// Sérialise une liste de cues en texte au format SRT.
pub fn to_srt(cues: &[Segment]) -> String {
    let mut out = String::new();
    for (i, cue) in cues.iter().enumerate() {
        out.push_str(&format!("{}\n", i + 1));
        out.push_str(&format!(
            "{} --> {}\n",
            format_timestamp(cue.start),
            format_timestamp(cue.end)
        ));
        out.push_str(cue.text.trim());
        out.push_str("\n\n");
    }
    out
}

/// Formate une durée en secondes au format SRT `HH:MM:SS,mmm`.
fn format_timestamp(seconds: f64) -> String {
    let total_ms = (seconds.max(0.0) * 1000.0).round() as u64;
    let ms = total_ms % 1000;
    let total_secs = total_ms / 1000;
    let secs = total_secs % 60;
    let total_mins = total_secs / 60;
    let mins = total_mins % 60;
    let hours = total_mins / 60;
    format!("{hours:02}:{mins:02}:{secs:02},{ms:03}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_timestamp_correctly() {
        assert_eq!(format_timestamp(0.0), "00:00:00,000");
        assert_eq!(format_timestamp(61.234), "00:01:01,234");
        assert_eq!(format_timestamp(3661.5), "01:01:01,500");
    }

    #[test]
    fn writes_srt_with_sequential_indices() {
        let cues = vec![
            Segment {
                start: 0.0,
                end: 1.5,
                text: "Bonjour.".into(),
            },
            Segment {
                start: 1.5,
                end: 3.0,
                text: "Comment ça va ?".into(),
            },
        ];
        let srt = to_srt(&cues);
        assert!(srt.starts_with("1\n00:00:00,000 --> 00:00:01,500\nBonjour.\n\n"));
        assert!(srt.contains("2\n00:00:01,500 --> 00:00:03,000\nComment ça va ?\n\n"));
    }
}
