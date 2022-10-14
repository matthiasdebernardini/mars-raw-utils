use crate::{constants, time};

use sciimg::error;

pub fn get_lmst() -> error::Result<time::MissionTime> {
    time::get_time(
        constants::time::NSYT_SOL_OFFSET,
        constants::time::NSYT_LONGITUDE,
        time::TimeSystem::LMST,
    )
}
