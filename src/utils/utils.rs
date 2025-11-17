use symphonia::core::units::Time;

pub fn format_time(time: Time) -> String {
    let sec = time.seconds;
    format!("{:02}:{:02}", sec / 60, sec % 60)
}
