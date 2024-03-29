use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConstellationDiscordError {
    #[error("No Channels? {0}")]
    _ChannelList(String),
}
