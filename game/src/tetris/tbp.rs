use super::*;
use enumset::{EnumSet, EnumSetType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum FrontendMessage {
    Rules {
        randomizer: String,
    },
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum BotMessage {
    Info {
        name: String,
        version: String,
        author: String,
        features: Vec<String>,
    },
    Ready,
    Error {
        reason: BotErrorReason,
    },
    Suggestion {
        moves: Vec<PieceState>,
        move_info: MoveInfo,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BotErrorReason {
    UnsupportedRules,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Start {
    pub board: ColoredBoard,
    pub queue: Vec<PieceKind>,
    pub hold: Option<PieceKind>,
    pub combo: u32,
    pub back_to_back: bool,
    #[serde(default)]
    pub randomizer: Randomizer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum Randomizer {
    SevenBag {
        #[serde(
            deserialize_with = "collect_enumset",
            serialize_with = "serialize_enumset"
        )]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

fn serialize_enumset<S, T>(set: &EnumSet<T>, ser: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
    T: EnumSetType + Serialize,
{
    set.iter().collect::<Vec<_>>().serialize(ser)
}
