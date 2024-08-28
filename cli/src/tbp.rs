use enumset::{EnumSet, EnumSetType};
use game::tetris::{BitBoard, PieceKind, PieceState};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum FrontendMessage {
    Rules,
    Start(Start),
    Play {
        #[serde(rename = "move")]
        mv: PieceState,
    },
    NewPiece {
        piece: PieceKind,
    },
    Suggest,
    Stop,
    Quit,
    #[serde(other)]
    Unknown,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum BotMessage {
    Info {
        name: &'static str,
        version: &'static str,
        author: &'static str,
        features: &'static [&'static str],
    },
    Ready,
    Suggestion {
        moves: Vec<PieceState>,
        move_info: MoveInfo,
    },
}

#[derive(Deserialize)]
pub struct Start {
    pub board: BitBoard,
    pub queue: Vec<PieceKind>,
    pub hold: Option<PieceKind>,
    pub combo: u32,
    pub back_to_back: bool,
    #[serde(default)]
    pub randomizer: Randomizer,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum Randomizer {
    SevenBag {
        #[serde(deserialize_with = "collect_enumset")]
        bag_state: EnumSet<PieceKind>,
    },
    #[serde(other)]
    Unknown,
}

impl Default for Randomizer {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Serialize)]
pub struct MoveInfo {
    pub nodes: u64,
    pub nps: f64,
    pub extra: String,
}

fn collect_enumset<'de, D, T>(de: D) -> Result<EnumSet<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: EnumSetType + Deserialize<'de>,
{
    Ok(Vec::<T>::deserialize(de)?.into_iter().collect())
}
