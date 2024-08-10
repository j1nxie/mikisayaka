use semver::Version;

const MADOKA_MAGICA: [&str; 12] = [
    "I First Met Her in a Dream... or Something.",
    "That Would Be Truly Wonderful",
    "I'm Not Afraid of Anything Anymore",
    "Miracles and Magic Are Real",
    "There's No Way I'll Ever Regret It",
    "This Just Can't Be Right",
    "Can You Face Your True Feelings?",
    "I Was Stupid... So Stupid",
    "I'd Never Allow That to Happen",
    "I Won't Rely On Anyone Anymore",
    "The Only Thing I Have Left to Guide Me",
    "My Very Best Friend",
];

pub fn get_version() -> String {
    let semver = env!("CARGO_PKG_VERSION").parse::<Version>();

    if let Ok(semver) = semver {
        let version_name = format!(
            "{} - {} [{}]",
            semver,
            MADOKA_MAGICA[(semver.major + semver.minor - 1) as usize],
            env!("VERGEN_GIT_SHA")
        );
        version_name
    } else {
        tracing::warn!("couldn't parse a semver out of Cargo.toml? defaulting to 0.0.0-unknown.");
        String::from("0.0.0-unknown - No Version Name")
    }
}