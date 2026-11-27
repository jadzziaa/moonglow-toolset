//! `mg.nwsync.status()` (API 0.11): what a plugin is told of the module's
//! NWSync content, from the module's project (its source, turned off, a
//! sign-in) and what the host knows (the manifest in the game data, how
//! much of its data is here, a Sync or a clean-up under way). Never the
//! sign-in itself.

use std::rc::Rc;
use std::time::{Duration, UNIX_EPOCH};

use mg_gff::Gff;
use mg_module::Module;
use mg_module::nwsync_content::{State, SurveyCounts};
use mg_module::project_file::ProjectFile;
use mg_plugin::{Input, run_console};

use crate::common::TestHost;

const URL: &str = "https://nwsync.example.org";
const TOKEN: &str = "s3cret-t0ken-of-the-test";

/// A module whose project is the files `team` (`moonglow.toml`) and `own`
/// (`.moonglow/user.toml`) in a folder of its own.
fn module_with(name: &str, team: &str, own: Option<&str>) -> Module {
    let root = mg_testkit::scratch_dir(name);
    std::fs::write(root.join("moonglow.toml"), team).unwrap();
    if let Some(own) = own {
        std::fs::create_dir_all(root.join(".moonglow")).unwrap();
        std::fs::write(root.join(".moonglow/.gitignore"), "*\n").unwrap();
        std::fs::write(root.join(".moonglow/user.toml"), own).unwrap();
    }
    let mut m = Module::new();
    m.set_info(&Gff::new(*b"IFO ")).unwrap();
    m.project_file = ProjectFile::read(&root);
    assert_eq!(m.project_file.warnings, Vec::<String>::new());
    m
}

fn team() -> String {
    format!("[nwsync]\nsource = \"{URL}\"\n")
}

/// What the console logs for `code` run on `input`.
fn said(code: &str, input: Input) -> Vec<String> {
    let host = Rc::new(TestHost::default());
    run_console(code, input, host.clone()).unwrap();
    host.log_texts()
}

/// A state of the content as the window has it: a manifest from the
/// game's storage that holds two haks, one found nowhere, a hak and the
/// talk table on disk newer than it, a look's counts.
fn known() -> State {
    State {
        source: URL.into(),
        manifest: Some("0123456789abcdef0123456789abcdef01234567".into()),
        from: Some("game_storage"),
        arrived: Some(UNIX_EPOCH + Duration::from_secs(1_700_000_000)),
        resources: Some(218_936),
        haks: vec!["pw_head".into(), "pw_tex".into()],
        by_origins: true,
        haks_missing: vec!["pw_gone".into()],
        newer_on_disk: vec!["pw_wip.hak".into(), "world.tlk".into()],
        survey: Some(SurveyCounts {
            resources: 218_936,
            data_files: 200_000,
            in_game_storage: 150_000,
            in_cache: 40_000,
            missing: 10_000,
            missing_bytes: 3_000_000_000,
        }),
        syncing: false,
        cleaning: false,
    }
}

/// The status's fields, one a line, as Luau reads them.
const FIELDS: &str = r#"
local s = mg.nwsync.status()
if s == nil then return "nil" end
local survey = s.survey and `{s.survey.resources} {s.survey.data_files} {s.survey.in_game_storage} {s.survey.in_cache} {s.survey.missing} {s.survey.missing_bytes}` or "no survey"
return `off={s.off} source={s.source} signed_in={s.signed_in}`,
    `manifest={s.manifest} from={s.from} arrived={s.arrived} resources={s.resources}`,
    `haks={table.concat(s.haks, ",")} by_origins={s.by_origins} missing={table.concat(s.haks_missing, ",")}`,
    `newer={table.concat(s.newer_on_disk, ",")}`,
    `survey={survey}`,
    `syncing={s.syncing} cleaning={s.cleaning}`
"#;

/// No source: nil. A source the host knows nothing of: the source alone,
/// no manifest, nothing under way.
#[test]
fn the_status_is_nil_without_a_source() {
    let m = module_with("plugin-nwsync-none", "", None);
    assert_eq!(said(FIELDS, Input::new(m, None)), ["\"nil\""]);
    // (What a host knows of no source is not told.)
    let m = module_with("plugin-nwsync-none-known", "", None);
    let input = Input { nwsync: Some(known()), ..Input::new(m, None) };
    assert_eq!(said(FIELDS, input), ["\"nil\""]);

    let m = module_with("plugin-nwsync-unknown", &team(), None);
    assert_eq!(
        said(FIELDS, Input::new(m, None)),
        [
            format!("\"off=false source={URL} signed_in=false\""),
            "\"manifest=nil from=nil arrived=nil resources=nil\"".into(),
            "\"haks= by_origins=false missing=\"".into(),
            "\"newer=\"".into(),
            "\"survey=no survey\"".into(),
            "\"syncing=false cleaning=false\"".into(),
        ]
    );
}

/// A source with its content in the game data: every field as the host
/// knows it, the files on disk newer than the manifest by their names.
#[test]
fn the_status_is_the_mounted_content() {
    let m = module_with("plugin-nwsync-mounted", &team(), None);
    let input = Input { nwsync: Some(known()), ..Input::new(m, None) };
    assert_eq!(
        said(FIELDS, input),
        [
            format!("\"off=false source={URL} signed_in=false\""),
            "\"manifest=0123456789abcdef0123456789abcdef01234567 from=game_storage \
             arrived=1700000000 resources=218936\""
                .into(),
            "\"haks=pw_head,pw_tex by_origins=true missing=pw_gone\"".into(),
            "\"newer=pw_wip.hak,world.tlk\"".into(),
            "\"survey=218936 200000 150000 40000 10000 3000000000\"".into(),
            "\"syncing=false cleaning=false\"".into(),
        ]
    );
    // What the host knows of another source (the project changed since)
    // is not this one's.
    let m = module_with("plugin-nwsync-other", &team(), None);
    let other = State { source: "https://other.example.org".into(), ..known() };
    let input = Input { nwsync: Some(other), ..Input::new(m, None) };
    assert_eq!(said(FIELDS, input)[1], "\"manifest=nil from=nil arrived=nil resources=nil\"");
}

/// A Sync under way, and a clean-up, as the job began.
#[test]
fn the_status_says_a_sync_or_a_clean_up_is_under_way() {
    for (syncing, cleaning) in [(true, false), (false, true)] {
        let m = module_with("plugin-nwsync-running", &team(), None);
        let state = State { syncing, cleaning, ..known() };
        let input = Input { nwsync: Some(state), ..Input::new(m, None) };
        let log = said(FIELDS, input);
        assert_eq!(log[5], format!("\"syncing={syncing} cleaning={cleaning}\""));
    }
}

/// `source = ""` in the builder's own file: off, the team's source named,
/// nothing else of it (what the host knew of it is not told).
#[test]
fn the_status_says_nwsync_is_turned_off() {
    let m = module_with("plugin-nwsync-off", &team(), Some("[nwsync]\nsource = \"\"\n"));
    let input = Input { nwsync: Some(known()), ..Input::new(m, None) };
    let log = said(FIELDS, input);
    assert_eq!(log[0], format!("\"off=true source={URL} signed_in=false\""));
    assert_eq!(log[1], "\"manifest=nil from=nil arrived=nil resources=nil\"");
}

/// Every string anywhere in the status (keys and values, nested tables
/// walked), one a line.
const EVERY_STRING: &str = r#"
local out = {}
local function walk(v)
    if type(v) == "string" then
        table.insert(out, v)
    elseif type(v) == "table" then
        for k, inner in v do
            walk(k)
            walk(inner)
        end
    end
end
walk(mg.nwsync.status())
return table.concat(out, "\n")
"#;

/// The sign-in in the builder's own file is never told: `signed_in` says
/// there is one for the source, and no string of the status holds the
/// token, nor does the console's print of the whole table.
#[test]
fn the_status_never_holds_the_sign_in() {
    let own = format!("[nwsync]\ntoken = \"{TOKEN}\"\nhost = \"nwsync.example.org\"\n");
    let input = || {
        let m = module_with("plugin-nwsync-sign-in", &team(), Some(&own));
        assert!(m.project_file.nwsync_auth.is_some(), "the sign-in is read");
        Input { nwsync: Some(known()), ..Input::new(m, None) }
    };
    let log = said(FIELDS, input());
    assert_eq!(log[0], format!("\"off=false source={URL} signed_in=true\""));
    let strings = said(EVERY_STRING, input()).join("\n");
    assert!(strings.contains(URL) && strings.contains("pw_wip.hak"), "{strings}");
    assert!(!strings.contains(TOKEN), "{strings}");
    let printed = said("return mg.nwsync.status()", input()).join("\n");
    assert!(printed.contains("signed_in") && !printed.contains(TOKEN), "{printed}");

    // A sign-in for another host is not this source's.
    let own = format!("[nwsync]\ntoken = \"{TOKEN}\"\nhost = \"other.example.org\"\n");
    let m = module_with("plugin-nwsync-sign-in-other", &team(), Some(&own));
    let log = said(FIELDS, Input::new(m, None));
    assert_eq!(log[0], format!("\"off=false source={URL} signed_in=false\""));
}
