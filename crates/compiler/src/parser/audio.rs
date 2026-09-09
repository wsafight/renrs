use super::{Cursor, Diagnostic, Line, Parser, StatementKind};

impl Parser {
    pub(super) fn parse_play(
        &mut self,
        line: &Line,
        cursor: &mut Cursor<'_>,
    ) -> Result<StatementKind, Diagnostic> {
        let target = cursor
            .identifier()
            .ok_or_else(|| self.error(line, cursor.column(), "expected `music` or `sound`"))?;
        let path = cursor
            .string()
            .map_err(|message| self.error(line, cursor.column(), message))?;
        let kind = match target.as_str() {
            "music" => {
                let (repeat, fade_in, volume, if_changed) =
                    self.parse_music_options(line, cursor)?;
                StatementKind::PlayMusic {
                    path,
                    repeat,
                    fade_in,
                    volume,
                    if_changed,
                }
            }
            "sound" => {
                let (repeat, volume) = self.parse_sound_options(line, cursor)?;
                StatementKind::PlaySound {
                    path,
                    volume,
                    repeat,
                }
            }
            _ => return Err(self.error(line, cursor.column(), "expected `music` or `sound`")),
        };
        cursor
            .end()
            .map_err(|message| self.error(line, cursor.column(), message))?;
        self.current += 1;
        Ok(kind)
    }

    pub(super) fn parse_queue(
        &mut self,
        line: &Line,
        cursor: &mut Cursor<'_>,
    ) -> Result<StatementKind, Diagnostic> {
        let target = cursor
            .identifier()
            .ok_or_else(|| self.error(line, cursor.column(), "expected `music` or `sound`"))?;
        let path = cursor
            .string()
            .map_err(|message| self.error(line, cursor.column(), message))?;
        let kind = match target.as_str() {
            "music" => {
                let (repeat, fade_in, volume, _) = self.parse_music_options(line, cursor)?;
                StatementKind::QueueMusic {
                    path,
                    repeat,
                    fade_in,
                    volume,
                }
            }
            "sound" => {
                let (repeat, volume) = self.parse_sound_options(line, cursor)?;
                StatementKind::QueueSound {
                    path,
                    volume,
                    repeat,
                }
            }
            _ => return Err(self.error(line, cursor.column(), "expected `music` or `sound`")),
        };
        cursor
            .end()
            .map_err(|message| self.error(line, cursor.column(), message))?;
        self.current += 1;
        Ok(kind)
    }

    pub(super) fn parse_voice(
        &mut self,
        line: &Line,
        cursor: &mut Cursor<'_>,
    ) -> Result<StatementKind, Diagnostic> {
        let path = cursor
            .string()
            .map_err(|message| self.error(line, cursor.column(), message))?;
        cursor
            .end()
            .map_err(|message| self.error(line, cursor.column(), message))?;
        self.current += 1;
        Ok(StatementKind::PlayVoice { path })
    }

    pub(super) fn parse_stop(
        &mut self,
        line: &Line,
        cursor: &mut Cursor<'_>,
    ) -> Result<StatementKind, Diagnostic> {
        let target = cursor.identifier().ok_or_else(|| {
            self.error(
                line,
                cursor.column(),
                "expected `music`, `sound`, or `voice`",
            )
        })?;
        let fade_out = if cursor.keyword("fadeout") {
            self.parse_audio_duration(line, cursor, "fadeout")?
        } else {
            0.0
        };
        cursor
            .end()
            .map_err(|message| self.error(line, cursor.column(), message))?;
        self.current += 1;
        Ok(match target.as_str() {
            "music" => StatementKind::StopMusic { fade_out },
            "sound" => StatementKind::StopSound { fade_out },
            "voice" => StatementKind::StopVoice { fade_out },
            _ => {
                return Err(self.error(
                    line,
                    cursor.column(),
                    "expected `music`, `sound`, or `voice`",
                ));
            }
        })
    }

    pub(super) fn parse_sound_options(
        &self,
        line: &Line,
        cursor: &mut Cursor<'_>,
    ) -> Result<(bool, f32), Diagnostic> {
        let mut repeat = false;
        let mut volume = 1.0;
        let mut has_volume = false;
        loop {
            if cursor.keyword("loop") && !repeat {
                repeat = true;
            } else if cursor.keyword("volume") && !has_volume {
                volume = self.parse_audio_volume(line, cursor)?;
                has_volume = true;
            } else {
                break;
            }
        }
        Ok((repeat, volume))
    }
}
