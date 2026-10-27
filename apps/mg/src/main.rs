//! `mg`: Moonglow's command-line tool for inspecting and converting game
//! files and the resources a game install provides.

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

mod claims;
mod diff;
mod edits;
mod lsp;
mod merge;
mod nui;
mod plugins;
mod search;
mod serve;
mod test;
mod tlk_alloc;
mod usage;
mod watch;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use mg_core::{Codepage, ResType, StrRef};
use mg_erf::{Erf, ErfWriter};
use mg_gff::Gff;
use mg_module::{Module, ModuleLocation};
use mg_resman::{GameInstall, ResKey, ResMan};

#[derive(Parser)]
#[command(name = "mg", version, about = "Moonglow Toolset command-line tools")]
struct Cli {
    /// Game install (default: $NWN_ROOT or the usual Steam location).
    #[arg(long, global = true)]
    root: Option<PathBuf>,
    /// NWN user directory (default: $NWN_HOME or the platform default).
    #[arg(long, global = true)]
    user_dir: Option<PathBuf>,
    /// Ignore the user directory (base game only).
    #[arg(long, global = true)]
    no_user_dir: bool,
    /// Print the result as one JSON object, for scripts and build
    /// pipelines (warnings in its "notes"; an error as {"error": …}).
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// List the contents of an ERF archive (mod, hak, erf, nwm, sav).
    Ls { archive: PathBuf },
    /// Unpack an ERF archive into a directory.
    Unpack { archive: PathBuf, out: PathBuf },
    /// Pack a directory's files into an ERF archive (type from the extension).
    Pack {
        dir: PathBuf,
        archive: PathBuf,
        /// Compile the models kept as text (ASCII .mdl) on the way in:
        /// each against its supermodel, from the directory or the game
        /// (found as for the other commands; without one, only the
        /// directory's). A model that doesn't compile, or whose
        /// supermodel isn't found, goes in as the text it is, and is named.
        #[arg(long)]
        compile_models: bool,
    },
    /// Convert GFF to JSON (neverwinter.nim / nasher format), or JSON to GFF.
    Gff {
        input: PathBuf,
        /// Output file (default: stdout for JSON).
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Show where a resource comes from in the game's load order: the
    /// layer the game reads it from, then every layer whose copy that
    /// overrides, highest first.
    Which {
        /// `name.ext`, e.g. `classes.2da`.
        resource: String,
        /// A module (.mod, folder or nasher project) whose haks, and the
        /// module itself, are in the load order too.
        #[arg(long)]
        module: Option<PathBuf>,
    },
    /// Print a resource from the game's load order to stdout.
    Cat {
        resource: String,
        /// A compiled model (.mdl) as the text it compiles from.
        #[arg(long)]
        text: bool,
    },
    /// List the resource layers in load order with their sizes.
    Layers,
    /// Print talk-table strings by StrRef.
    Tlk { strrefs: Vec<u32> },
    /// Check a module: references to resources that exist nowhere, and its
    /// custom content's problems (tilesets, 2DAs, materials, objects naming
    /// rows that don't exist). Exits with an error if there are errors.
    Verify {
        /// A module archive, folder or nasher project.
        module: PathBuf,
        /// Also list module resources nothing references.
        #[arg(long)]
        unused: bool,
        /// Run these plugins' checks too: a plugin's folder, or a folder
        /// of plugins.
        #[arg(long)]
        plugins: Vec<PathBuf>,
        /// Leave a group out: its checks aren't run (repeatable, or
        /// several separated by commas): references, tilesets, models,
        /// 2das, objects, module or plugins (`mg checks` names each
        /// check's group).
        #[arg(long, value_name = "GROUP", value_parser = verify_group, value_delimiter = ',')]
        skip: Vec<mg_module::doctor::Group>,
        /// Make the fixes of this check's findings and save (repeatable):
        /// a check's id, as a finding's "check" with --json says it
        /// (`tag-unique`, `example.tag-conventions/tag-case`). Only the
        /// findings that offer a fix (with --json, a finding's "fix").
        #[arg(long, value_name = "CHECK")]
        fix: Vec<String>,
        /// With --fix, print the fixes it would make; change nothing.
        #[arg(long, requires = "fix")]
        dry_run: bool,
        /// Run the checks of every plugin the project comes with (its
        /// `plugins/`), not only those you allowed in Moonglow: for a
        /// build pipeline, where nobody is asked. Without it, a project's
        /// plugin runs only as you allowed it (`.moonglow/user.toml`), and
        /// one not allowed is said.
        #[arg(long)]
        allow_project_plugins: bool,
        /// Print how long each part of the check took, to stderr.
        #[arg(long)]
        timings: bool,
    },
    /// Plugins: list them, check them, run one's command on a module.
    Plugin {
        #[command(subcommand)]
        cmd: PluginCmd,
    },
    /// NUI windows, as the NUI Creator makes them: add one to a module,
    /// check one, write and compile one's scripts.
    Nui {
        #[command(subcommand)]
        cmd: nui::NuiCmd,
    },
    /// List the checks `verify` makes of custom content: each one's id (a
    /// finding's "check" with --json) and what it holds to be true.
    Checks,
    /// The variables a project describes (`variables.toml` beside its
    /// `moonglow.toml`): each one's type, what it goes on, its values (a
    /// script's constants and a table's rows read) and its help. `verify`
    /// holds the module's variables to them.
    Vars {
        /// The module: a folder or a nasher project.
        module: PathBuf,
        /// Only those that go on this kind of object (`placeable`,
        /// `creature`, `area`, `module`...).
        #[arg(long)]
        on: Option<String>,
        /// Only this variable (or those a glob names).
        name: Option<String>,
    },
    /// Show, set or take out your builder code in a project: the ResRefs
    /// the window proposes for new areas, blueprints and copies begin with
    /// it (by the team's `[resrefs] pattern`). It is kept in the project's
    /// `.moonglow/user.toml`, which version control leaves out.
    BuilderCode {
        /// The project: a module folder, a nasher project's folder or its
        /// nasher.cfg.
        project: PathBuf,
        /// The code to set (letters, digits and _, at most 8).
        code: Option<String>,
        /// Take the code out.
        #[arg(long, conflicts_with = "code")]
        clear: bool,
    },
    /// The project's claims table (moonglow.toml's `[claims] file`): who
    /// is building which area. Lists every area of the table and the
    /// module, whose each is; claims areas by your builder code, releases
    /// your claims, or marks areas merged (in the game: their Tag, recorded
    /// in the table, and ResRef are kept). The table is a file of the
    /// checkout: commit it.
    Claims {
        /// The project: a module folder, a nasher project's folder or its
        /// nasher.cfg.
        project: PathBuf,
        /// list, claim, release or merged.
        #[arg(value_enum, default_value = "list")]
        action: claims::Action,
        /// The areas (their ResRefs).
        areas: Vec<String>,
        /// Claim or release an area another builder holds.
        #[arg(long)]
        force: bool,
    },
    /// Report the resources a module's haks provide, their conflicts and the
    /// base-game resources they override.
    Haks {
        module: PathBuf,
        /// Which of each hak's resources the module reaches (its areas'
        /// tilesets and tiles, its objects' and blueprints' models, parts,
        /// portraits and sounds, its scripts and what they name, and the
        /// textures, materials and supermodels of every model), and which
        /// nothing reaches: a line a hak, used of all and bytes used of
        /// all. With --json, each resource used with the module resource
        /// that reached it first, and those unused. The manual's Modules
        /// chapter says what is followed and what is not.
        #[arg(long)]
        usage: bool,
    },
    /// Export resources and their dependencies from a module to an ERF; or,
    /// with --files, the resources named as loose files in a folder (for
    /// the game's `development` or `override`, or to hand on).
    Export {
        module: PathBuf,
        /// Resources to export (`name.ext`; an area brings its .git/.gic).
        #[arg(required = true)]
        resources: Vec<String>,
        /// The ERF to write; with --files, the folder to write into.
        #[arg(short, long)]
        output: PathBuf,
        /// Write the resources named as files (`name.ext`) in the folder
        /// OUTPUT, each with what goes with it (a script's compiled .ncs
        /// and .ndb, an area's .git and .gic), as they are in the module;
        /// not what they depend on.
        #[arg(long)]
        files: bool,
        /// Description stored in the ERF. What the export was made from
        /// follows it there: the module's git commit (and whether files
        /// had changes not committed), Moonglow's version and the module's
        /// name, which `mg import` reads back.
        #[arg(long, default_value = "")]
        comment: String,
        /// Keep custom factions instead of resetting them to their parents.
        #[arg(long)]
        keep_factions: bool,
    },
    /// Compile a module's scripts and save it (Build › Compile).
    Compile {
        module: PathBuf,
        /// Only scripts without a compiled version.
        #[arg(long)]
        uncompiled: bool,
        /// Every script, also those that are as they were when last
        /// compiled (by `mg compile` or in the window: both remember, in
        /// Moonglow's data folder, what they compiled).
        #[arg(long, conflicts_with = "uncompiled")]
        all: bool,
        /// Make each compiled script's debug file (.ndb) too; a compiled
        /// script that has none is compiled again for it.
        #[arg(long)]
        ndb: bool,
    },
    /// Run a module in the game's headless server (nwserver): privately
    /// unless --public, on a scratch user directory unless --dir, its log
    /// and console printed. What you type goes to the server's console
    /// (`status`, `exit`…); `/restart` starts it again and `/quit` stops
    /// it. With --json, each line printed is one JSON event.
    Serve {
        /// A .mod, a module folder or a nasher project (packed for the
        /// server, which loads an archive only).
        module: PathBuf,
        /// The server's user directory (its vault, database, logs and
        /// settings). Default: one of Moonglow's in the temporary folder,
        /// the same for each run of this module.
        #[arg(long)]
        dir: Option<PathBuf>,
        /// Allow --dir to be your own NWN folder.
        #[arg(long)]
        real_user_dir: bool,
        /// The port (default: a free one).
        #[arg(long)]
        port: Option<u16>,
        /// List the server publicly (default: not listed).
        #[arg(long)]
        public: bool,
        /// The server's name.
        #[arg(long, default_value = "Moonglow test server")]
        name: String,
        /// The player password (default: a random one, printed).
        #[arg(long)]
        player_password: Option<String>,
        /// The DM password (default: a random one, printed).
        #[arg(long)]
        dm_password: Option<String>,
        /// The admin password (default: a random one).
        #[arg(long)]
        admin_password: Option<String>,
        /// A file of NAME=value lines added to the server's environment
        /// (NWNX's settings, say); may be given more than once.
        #[arg(long)]
        env: Vec<PathBuf>,
        /// A library loaded into the server (LD_PRELOAD; Linux).
        #[arg(long)]
        preload: Option<PathBuf>,
        /// Mark the log lines containing this text (an event of their own
        /// with --json); may be given more than once.
        #[arg(long)]
        marker: Vec<String>,
        /// Stop the server once a log line contains this text.
        #[arg(long)]
        until: Option<String>,
        /// Stop after this many seconds (a failure if --until was not
        /// seen).
        #[arg(long)]
        timeout: Option<f64>,
        /// Start the server again when the module's file changes.
        #[arg(long)]
        restart_on_change: bool,
        /// More arguments for nwserver, after `--`.
        #[arg(last = true)]
        server_args: Vec<String>,
    },
    /// Run a module's NWScript tests in the game's server: its scripts
    /// named test_*, each with a main(), that include "mg_test" and call
    /// Assert(), AssertEqualInt()… (`mg test --include` prints it). They
    /// run in a scratch copy of the module, which gets a driver as its
    /// OnModuleLoad script; the module itself is not changed. Fails if a
    /// test fails or does not run to its end.
    Test {
        /// A .mod, a module folder or a nasher project.
        module: Option<PathBuf>,
        /// Only the tests whose name has this text (`*` for any run of
        /// characters: `test_rules_*`).
        #[arg(long)]
        filter: Option<String>,
        /// A folder of more test scripts (.nss), added to the copy; may be
        /// given more than once.
        #[arg(long)]
        tests: Vec<PathBuf>,
        /// `none`: the module runs its tests itself (from NWNX, say) and
        /// calls TestsDone(); nothing is put into the copy.
        #[arg(long, default_value = "mg", value_parser = ["mg", "none"])]
        driver: String,
        /// Seconds to wait after the last test was started, for what tests
        /// assert later (DelayCommand).
        #[arg(long, default_value_t = 0.0)]
        settle: f32,
        /// Give up after this many seconds.
        #[arg(long, default_value_t = 120.0)]
        timeout: f64,
        /// Also write the results as a JUnit report (XML).
        #[arg(long)]
        junit: Option<PathBuf>,
        /// Print the server's log as it comes (to standard error).
        #[arg(long)]
        log: bool,
        /// Print the assertion include (mg_test.nss) and do nothing else.
        #[arg(long)]
        include: bool,
        /// The server's user directory (default: a scratch one).
        #[arg(long)]
        dir: Option<PathBuf>,
        /// A file of NAME=value lines for the server's environment.
        #[arg(long)]
        env: Vec<PathBuf>,
        /// A library loaded into the server (LD_PRELOAD; Linux).
        #[arg(long)]
        preload: Option<PathBuf>,
        /// More arguments for nwserver, after `--`.
        #[arg(last = true)]
        server_args: Vec<String>,
    },
    /// What differs between two 2DA tables, by row number and column
    /// name. With --base (the table both were made from), the two merged:
    /// what either changed is taken, and a cell both changed, each to
    /// something else, is a conflict, which fails unless --prefer.
    #[command(name = "2da-diff")]
    TwodaDiff {
        a: PathBuf,
        b: PathBuf,
        /// The table A and B were both made from: merge them.
        #[arg(long)]
        base: Option<PathBuf>,
        /// Write the merged table here.
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Write an edit file for `mg apply` that sets the table in a
        /// module to B (or, with --base, to the merged table).
        #[arg(long)]
        edits: Option<PathBuf>,
        /// Whose value a conflict takes: a or b.
        #[arg(long)]
        prefer: Option<String>,
        /// The table's name in the edit file (default: B's file name).
        #[arg(long)]
        resource: Option<String>,
    },
    /// Check builders' own 2DAs against their descriptions: a file
    /// `name.2da.toml` beside `name.2da` that says what each column holds
    /// (a whole number in a range, one of a list of values, a set of
    /// flags, a row of another table, a talk-table string) and whose rows
    /// the table's are. Lists may be a script's constants, read from the
    /// script. Each path is a table, or a folder: every table in it (and
    /// in the folders in it) that has a description. Scripts and tables a
    /// description names are looked for in the folder (or --beside),
    /// then in the game. Fails if a cell is wrong or a description cannot
    /// be read. The format: the manual's Modules chapter, "A table's
    /// description"
    #[command(name = "2da-check")]
    TwodaCheck {
        /// Tables (.2da) or folders.
        #[arg(required = true)]
        paths: Vec<PathBuf>,
        /// A folder whose scripts and tables the descriptions may name
        /// (default: the folder given; for a table, the repository or
        /// nasher project it is in, else its own folder)
        #[arg(long)]
        beside: Option<PathBuf>,
    },
    /// What differs between two talk tables (.tlk), by line. With --base
    /// (the table both were made from), the two merged, as `mg 2da-diff`
    /// merges.
    TlkDiff {
        a: PathBuf,
        b: PathBuf,
        /// The table A and B were both made from: merge them.
        #[arg(long)]
        base: Option<PathBuf>,
        /// Write the merged table here.
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Write the lines that differ as a file for `mg tlk-import`
        /// (.json): B's (or, with --base, the merged table's where it
        /// differs from the base).
        #[arg(long)]
        lines: Option<PathBuf>,
        /// Whose line a conflict takes: a or b.
        #[arg(long)]
        prefer: Option<String>,
    },
    /// Merge two copies of a GFF resource (an area's .git and .are,
    /// module.ifo, a blueprint, a conversation), both made from BASE: a
    /// value one side changed is taken, the same change on both is one,
    /// and two different changes are a conflict, each named by its path
    /// (the merge fails and nothing is written, unless --prefer takes a
    /// side). Lists merge by item: an area's placed objects by object
    /// (added by both: both kept), the area list as a union, variables by
    /// name and type, feats, classes, spells, haks and journal entries by
    /// what they are; other lists by place. Each file is a GFF or nasher's JSON of
    /// one, told by its contents. The merge is written over OURS (or -o),
    /// in OURS's form, and a merge that is OURS leaves the file as it was:
    /// git's merge driver (`mg merge-gff %O %A %B --path %P`; the manual's
    /// Modules chapter, "Merging in git", has the lines for .gitattributes
    /// and .git/config)
    #[command(name = "merge-gff")]
    MergeGff {
        /// The file both were made from (git's %O).
        base: PathBuf,
        /// Ours (git's %A): the merge is written here.
        ours: PathBuf,
        /// Theirs (git's %B).
        theirs: PathBuf,
        /// Whose value a conflict takes: ours or theirs (then the merge
        /// is written, and succeeds).
        #[arg(long)]
        prefer: Option<String>,
        /// Write the merge here instead of over OURS.
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// The file's path in the repository (git's %P): its name in the
        /// report, and where its nasher project is looked for (the float
        /// places and codepage JSON is written with; default: OURS's
        /// folder).
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Write a custom talk table from strings kept by name (a .toml or
    /// .json file: `key = "text"`, in groups if you like), each on a line
    /// that stays its own: a map kept beside the names file says which. A
    /// new key gets the next line that was never anyone's; a key taken
    /// away keeps its line reserved.
    TlkAlloc {
        /// The talk table (made if it isn't there).
        table: PathBuf,
        /// The strings by name.
        #[arg(long)]
        from: PathBuf,
        /// The map of keys to lines (default: the names file's name with
        /// `.strrefs.json`). Keep it with the project.
        #[arg(long)]
        map: Option<PathBuf>,
        /// Write the keys as constants with the game's numbers
        /// (16777216 and up): an NWScript include (.nss) or .json.
        #[arg(long)]
        constants: Option<PathBuf>,
        /// What a constant's name begins with.
        #[arg(long, default_value = "STRREF_")]
        prefix: String,
        /// The first line new keys may take.
        #[arg(long, default_value_t = 0)]
        start: u32,
        /// Write nothing: fail unless the table, the map and the
        /// constants are as they would be written.
        #[arg(long)]
        check: bool,
        /// An older map (the main branch's, say): fail if a key has
        /// another line now than there.
        #[arg(long)]
        against: Option<PathBuf>,
    },
    /// Follow a module kept as files (a nasher project or a module
    /// folder) while you edit it: a changed script is compiled, with the
    /// scripts that include it, and put into a development folder with
    /// changed blueprints and conversations, where a running server
    /// takes them up at their next use. With --serve the module is also
    /// run in the server, which is started again, on the module built
    /// anew, when something changes that it reads only at its start (a
    /// 2DA, an area, module.ifo).
    Watch {
        project: PathBuf,
        /// The development folder to write into (default with --serve:
        /// the server's own).
        #[arg(long)]
        dev_dir: Option<PathBuf>,
        /// Also build the module to this file (.mod), at the start and
        /// after every change.
        #[arg(long, value_name = "FILE")]
        r#mod: Option<PathBuf>,
        /// Make each compiled script's debug file (.ndb) too, for the
        /// game's script debugger.
        #[arg(long)]
        ndb: bool,
        /// Run the module in the server too, as `mg serve` does.
        #[arg(long)]
        serve: bool,
        /// With --serve: the server's user directory (default: a scratch
        /// one).
        #[arg(long)]
        dir: Option<PathBuf>,
        /// With --serve: the port (default: a free one).
        #[arg(long)]
        port: Option<u16>,
        /// With --serve: a file of NAME=value lines for the server's
        /// environment.
        #[arg(long)]
        env: Vec<PathBuf>,
        /// With --serve: a library loaded into the server (LD_PRELOAD).
        #[arg(long)]
        preload: Option<PathBuf>,
        /// With --serve: more arguments for nwserver, after `--`.
        #[arg(last = true)]
        server_args: Vec<String>,
    },
    /// An NWScript language server (the Language Server Protocol on standard
    /// input and output) for VS Code, Neovim and other editors: errors as you
    /// type, definitions, references, rename, hover, completion, outline.
    Lsp {
        /// The codepage scripts are read in, as the game reads their text:
        /// cp1252, cp1250 (Polish) or cp1251. Default: the game's own
        /// setting in the user directory's settings.tml (its codepage, else
        /// its language's), else cp1252.
        #[arg(long)]
        codepage: Option<String>,
    },
    /// Where a resource (or, with --tag, a tag) is used in a module, and the
    /// script strings that spell it.
    Refs {
        module: PathBuf,
        /// `name.ext`, e.g. `guard_spawn.nss`, `keep.are`, `guard.utc`.
        name: String,
        /// Look for a tag instead of a resource.
        #[arg(long)]
        tag: bool,
    },
    /// Rename a script, area, conversation or blueprint everywhere in a
    /// module and save it: what names it follows, scripts whose source
    /// changed are compiled again.
    Rename {
        module: PathBuf,
        /// `name.ext` of the resource to rename.
        from: String,
        /// The new name (without extension).
        to: String,
        /// Also change script strings that spell the old name.
        #[arg(long)]
        strings: bool,
    },
    /// Write a conversation as plain text, CSV, Twine (Twee) or Ink, by the
    /// output's extension (.txt, .csv, .twee, .ink).
    DialogExport {
        module: PathBuf,
        /// The conversation, `name.dlg`.
        dialog: String,
        output: PathBuf,
    },
    /// Read a Twine (.twee) or Ink (.ink) story into a module as a
    /// conversation (replacing one of the same name) and save; or, from a
    /// .csv export, read back the text of the conversation's lines.
    DialogImport {
        module: PathBuf,
        file: PathBuf,
        /// The conversation's name (default: the file's).
        #[arg(long)]
        name: Option<String>,
    },
    /// Write a talk table (a .tlk file) as JSON, as nwn_tlk writes it, or as
    /// CSV (a row a line: StrRef, text, the feminine table's text if a
    /// table named with an added `f` lies beside it, sound, its length),
    /// by the output's extension (.json, .csv). A CSV's StrRefs are a
    /// custom table's, as a module's files name its lines: 16777216 and
    /// up (line 131 is 16777347). For the game's own dialog.tlk, --base
    /// writes each line's own number. (JSON has the line's number either
    /// way, as nwn_tlk does.)
    TlkExport {
        table: PathBuf,
        output: PathBuf,
        /// CSV: number the lines from 0, as the game's dialog.tlk's are
        /// named, not as a custom table's (`mg tlk-import` reads both)
        #[arg(long)]
        base: bool,
    },
    /// Read a .json or .csv file of talk-table lines into a talk table (a
    /// .tlk file, made if it isn't there) and save it: the lines the file
    /// has are set, the others stay. Nothing is written unless every line
    /// can be read.
    TlkImport {
        table: PathBuf,
        file: PathBuf,
        /// Say what would change, and write nothing.
        #[arg(long)]
        dry_run: bool,
    },
    /// Replace text in a module's names, descriptions, conversation lines
    /// and journal (every language they're written in) and save; prints
    /// each string changed.
    Replace {
        module: PathBuf,
        find: String,
        with: String,
        #[arg(long)]
        match_case: bool,
        /// Only where it isn't part of a longer word.
        #[arg(long)]
        whole_word: bool,
        /// Only these kinds: names, descriptions, conversations, journal,
        /// other (comma-separated).
        #[arg(long, value_delimiter = ',')]
        only: Vec<String>,
        /// List what would change; change nothing.
        #[arg(long)]
        dry_run: bool,
    },
    /// List a module's areas, or set properties of several at once and
    /// save. Which areas: those named, narrowed by the filters (all that are
    /// given must hold). With `--set`, `--var`, `--remove-var` or the
    /// grass options, each change is made to each of them: all of the
    /// changes, or none if one cannot be made.
    Areas {
        module: PathBuf,
        /// Only these areas (ResRefs); none: every area.
        areas: Vec<String>,
        /// Areas with this text in their name, tag or ResRef.
        #[arg(long = "match")]
        text: Option<String>,
        /// Areas of this tileset (its ResRef, e.g. `tdc01`).
        #[arg(long)]
        tileset: Option<String>,
        #[arg(long, conflicts_with = "exterior")]
        interior: bool,
        #[arg(long)]
        exterior: bool,
        #[arg(long, conflicts_with = "above_ground")]
        underground: bool,
        #[arg(long)]
        above_ground: bool,
        #[arg(long, conflicts_with = "artificial")]
        natural: bool,
        #[arg(long)]
        artificial: bool,
        /// `FIELD=VALUE`: a field of the area (`SunFogAmount=5`,
        /// `MoonAmbientColor=0x402010`, `OnEnter=my_script`), of its
        /// ambient sounds and music (`MusicDay=57`), or a flag
        /// (`Interior`, `Underground`, `Natural`: `yes` or `no`). A field
        /// keeps its type; a number may be hexadecimal (`0x…`).
        #[arg(long = "set")]
        sets: Vec<String>,
        /// `NAME=VALUE` or `NAME:int=VALUE` (`int`, `float`, `string`): a
        /// scripting variable to set on each area, which keeps its others.
        #[arg(long = "var")]
        vars: Vec<String>,
        /// A scripting variable to delete from each area.
        #[arg(long)]
        remove_var: Vec<String>,
        /// Make each area's placeables static, where nothing is lost by
        /// it: not those that are Useable, are tilted, scaled or moved by
        /// a visual transform, or have a script, a conversation, a trap,
        /// an inventory or an animation switched on. The game loads and
        /// draws static placeables more cheaply.
        #[arg(long, conflicts_with = "dynamic_placeables")]
        static_placeables: bool,
        /// Make every static placeable of each area dynamic.
        #[arg(long)]
        dynamic_placeables: bool,
        /// `MATERIAL:TEXTURE:DENSITY:HEIGHT[:R,G,B[:R,G,B]]`: grass of
        /// each area's own on a surface material (a surfacemat.2da row, by
        /// number or label: `Dirt:grass:3:0.5`), in place of what grows
        /// there, as `SetAreaGrassOverride` sets it, with an ambient and a
        /// diffuse color (0 to 1; the diffuse the ambient's when left out,
        /// both the game's grass color when neither is given). Kept in the
        /// GIT's AreaProperties, where the game reads it.
        #[arg(long = "grass", value_name = "GRASS")]
        grass: Vec<String>,
        /// A surface material (number or label) to take each area's own
        /// grass off.
        #[arg(long, value_name = "MATERIAL")]
        remove_grass: Vec<String>,
        /// `yes` or `no`: the tileset's grass grows on grass faces or not
        /// (`SetAreaDefaultGrassDisabled`).
        #[arg(long, value_name = "YES_OR_NO")]
        tileset_grass: Option<String>,
        /// A lighting scheme (an environment.2da row, by number, label or
        /// name: `Crypt`), as Area Properties' Lighting Scheme gives it:
        /// the area's sun and moon, fog, shadows, day and night and
        /// weather, and each tile's lights picked at random from the
        /// scheme's colors. Reads the game's data (and the module's haks).
        /// `--set` changes made with it are made after it.
        #[arg(long, value_name = "SCHEME")]
        lighting_scheme: Option<String>,
        /// Change every area of the module (needed when no area is named
        /// and no filter given).
        #[arg(long)]
        all: bool,
        /// List what would change; change nothing.
        #[arg(long)]
        dry_run: bool,
    },
    /// Make the objects placed from blueprints again from them, where they
    /// stand (Aurora's Update Instances), and save: their tags, names,
    /// scripts and variables become the blueprints'.
    UpdateInstances {
        module: PathBuf,
        /// `name.ext` of each blueprint, e.g. `guard.utc`; none: every
        /// blueprint in the module.
        blueprints: Vec<String>,
        /// Only in this area.
        #[arg(long)]
        area: Option<String>,
    },
    /// Build a nasher project's module: compile its scripts and pack its
    /// target as nasher does (filters, modName and the rest). Fails if a
    /// script doesn't compile, unless --keep-going. A target that packs a
    /// talk table (a .tlk) makes it of its <name>.tlk.json, as nasher and
    /// nwn_tlk do.
    Build {
        /// The project's folder (with its nasher.cfg).
        project: PathBuf,
        /// The target to pack (default: the target that packs a module).
        #[arg(long)]
        target: Option<String>,
        /// Where to write it (default: the target's file in the project).
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Pack even if scripts fail to compile (as nasher does by default).
        #[arg(long)]
        keep_going: bool,
        /// Make each compiled script's debug file (.ndb) too, packed with
        /// it.
        #[arg(long)]
        ndb: bool,
    },
    /// Put a module into a nasher project: its resources as source files
    /// (GFF as JSON), with a nasher.cfg like `nasher init` writes unless the
    /// folder has one.
    Init {
        /// A module archive or folder.
        module: PathBuf,
        /// The project's folder.
        project: PathBuf,
    },
    /// Attach haks and a talk table to a module: copies them into the user
    /// folder's hak and tlk (where the game looks), lists the haks at the
    /// top of the module's hak list in the order given, names the talk
    /// table, and saves the module.
    Attach {
        module: PathBuf,
        /// .hak and .tlk files, highest priority first.
        #[arg(required = true)]
        files: Vec<PathBuf>,
        /// Replace a different file of the same name in the user folder.
        #[arg(long)]
        replace: bool,
    },
    /// Write an area's minimap as a PNG: each tile's picture, turned as the
    /// tile is, as the game's map shows it.
    Minimap {
        module: PathBuf,
        /// The area's resref.
        area: String,
        /// The PNG to write.
        out: PathBuf,
        /// Pixels a tile (default: the pictures' size).
        #[arg(long)]
        size: Option<u32>,
    },
    /// Find a module's blueprints and the objects placed in its areas, by
    /// type, tag, name, resref, area or field values.
    Find {
        module: PathBuf,
        /// Blueprint types: utc, utd, ute, uti, utp, uts, utm, utt, utw
        /// (comma-separated; default all).
        #[arg(long = "type", value_delimiter = ',')]
        types: Vec<String>,
        /// The tag (`*` matches any run of characters; case ignored).
        #[arg(long)]
        tag: Option<String>,
        /// Words the name contains.
        #[arg(long)]
        name: Option<String>,
        /// The blueprint's resref (a placed object's blueprint); `*` as in tags.
        #[arg(long)]
        resref: Option<String>,
        /// Only objects placed in this area.
        #[arg(long)]
        area: Option<String>,
        /// Only placed objects.
        #[arg(long, conflicts_with = "blueprints")]
        placed: bool,
        /// Only blueprints.
        #[arg(long)]
        blueprints: bool,
        /// A field and its value, `Label=Value` (`*` as in tags), or `Label`
        /// for any value; repeatable.
        #[arg(long = "where")]
        fields: Vec<String>,
    },
    /// Find a module's blueprints, placed objects, areas and the rest by
    /// what they hold: variables, item properties, classes, feats, spells,
    /// skills, appearances, scripts, or any field's label or value.
    Search(search::Args),
    /// What a module is: its name, tag, areas, haks, talk table and
    /// resources by type.
    Info { module: PathBuf },
    /// Write the files the game draws something with into a folder: its
    /// models, their walkmeshes (.pwk, .dwk), textures, materials (.mtr,
    /// .txi), the models its emitters throw and the model its animations
    /// come from, from the haks and wherever else in the load order they
    /// are. The game's own files are left out, unless all are the game's.
    ModelFiles {
        /// A model's name (plc_a08), or a blueprint (chest.utp; also .utd,
        /// .uti, .utc), in the module if one is given, else in the game.
        what: String,
        /// The folder to write into (made if missing).
        out: PathBuf,
        /// The module whose haks (and blueprints) to look in.
        #[arg(long)]
        module: Option<PathBuf>,
        /// The game's own files too.
        #[arg(long)]
        all: bool,
        /// Write over files of the same names in the folder.
        #[arg(long)]
        force: bool,
        /// Only list the files; write nothing.
        #[arg(long)]
        dry_run: bool,
    },
    /// Check that Moonglow keeps a module as it is: saves a copy (the
    /// module is not touched), opens it again and compares every resource
    /// byte for byte; then writes every GFF and 2DA anew and compares what
    /// it reads back. Exits with an error if anything differs.
    Roundtrip {
        /// A .mod file or a module folder.
        module: PathBuf,
        /// Keep the copy here (a .mod file) rather than a temporary one.
        #[arg(long)]
        keep: Option<PathBuf>,
    },
    /// Make a tileset's palette (<tileset>palstd.itp) from its .set: its
    /// groups, features, terrains and crossers, as the painter offers them.
    TilesetPalette {
        /// The .set file.
        set: PathBuf,
        /// Where to write it (default: beside the .set).
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Publish a module's haks and talk table for NWSync: write a manifest
    /// and its data into a repository folder, for a web server to serve and
    /// nwserver to name (-nwsyncurl), as nwn_nwsync_write does.
    Nwsync {
        module: PathBuf,
        /// The repository folder (made if missing).
        repository: PathBuf,
        /// The module's own resources too, for single-player distribution
        /// (not for persistent worlds).
        #[arg(long)]
        with_module: bool,
        /// The module's UUID, with --with-module (default: its own, or a
        /// new one).
        #[arg(long)]
        uuid: Option<String>,
        /// The name players see (default with --with-module: the module's).
        #[arg(long)]
        name: Option<String>,
        /// The description players see (default with --with-module: the
        /// module's).
        #[arg(long)]
        description: Option<String>,
        /// For servers sharing a repository: the game removes a group's
        /// older downloads.
        #[arg(long, default_value_t = 0)]
        group_id: u32,
        /// Don't point `latest` at the new manifest.
        #[arg(long)]
        no_latest: bool,
        /// The largest file allowed, in megabytes (0: no limit).
        #[arg(long, default_value_t = 15)]
        limit_file_size: u64,
        /// Write data files that are there already again.
        #[arg(long)]
        force: bool,
        /// Work it out; write nothing.
        #[arg(long)]
        dry_run: bool,
    },
    /// Import an ERF into a module and save it. What the ERF was made
    /// from (stamped by Moonglow's export) is said, with a warning when
    /// the module's git checkout is at another commit now: the ERF was
    /// made from older work, and copying it in undoes what changed since.
    Import {
        module: PathBuf,
        erf: PathBuf,
        /// Overwrite resources the module already has.
        #[arg(long)]
        overwrite: bool,
    },
    /// Set fields of one of a module's resources and save: each
    /// `FIELD=VALUE`, the field a path from the resource's root (`Tag`,
    /// `/ClassList[0]/ClassLevel`). A field keeps its type; one the
    /// resource lacks takes the game's for it, or the one given
    /// (`FIELD:TYPE=VALUE`).
    Set {
        module: PathBuf,
        /// `name.ext`, e.g. `guard.utc`.
        resource: String,
        /// `FIELD=VALUE`, or `FIELD:TYPE=VALUE`.
        fields: Vec<String>,
        /// Remove this field.
        #[arg(long)]
        remove: Vec<String>,
        /// Check the change and print it; change nothing.
        #[arg(long)]
        dry_run: bool,
        /// Plugins whose handlers look at what is saved (`resource-saving`:
        /// a refusal stops the save, a warning is said; `resource-saved`,
        /// after): a plugin's folder, or a folder of plugins. The project's
        /// own plugins do too, as you allowed them.
        #[arg(long)]
        plugins: Vec<PathBuf>,
    },
    /// Apply a file of edits (JSON: fields set and removed, list items
    /// inserted and removed, resources written and removed) to a module and
    /// save: all of them, or none if one does not apply. `-` reads standard
    /// input.
    Apply {
        module: PathBuf,
        edits: PathBuf,
        /// Check that the edits apply; change nothing.
        #[arg(long)]
        dry_run: bool,
        /// Plugins whose handlers look at what is saved, as for `mg set`.
        #[arg(long)]
        plugins: Vec<PathBuf>,
    },
}

/// `mg plugin`'s commands.
// (Parsed once, from the command line: Run's many arguments cost nothing.)
#[allow(clippy::large_enum_variant)]
#[derive(Subcommand)]
enum PluginCmd {
    /// List plugins and what each adds: its commands, checks, events and
    /// settings. A folder is a plugin's own, or one that holds plugins'
    /// folders.
    List {
        folders: Vec<PathBuf>,
        /// Also the plugins this project comes with (a module folder, a
        /// nasher project's folder or its nasher.cfg): each with whether
        /// you allowed it, and its settings' values in the project.
        #[arg(long)]
        project: Option<PathBuf>,
    },
    /// Allow a project's own plugins to run, as they are now (or, with
    /// --deny, keep them off), as Moonglow asks when the project is opened.
    /// The answer is kept in Moonglow's own folder, by the project, the
    /// plugin and a digest of its files (a changed plugin is asked about
    /// again), never in the project.
    Allow {
        /// The project: a module folder, a nasher project's folder or its
        /// nasher.cfg.
        project: PathBuf,
        /// The plugins' ids (`mg plugin list --project` shows them); none:
        /// every one of the project's.
        ids: Vec<String>,
        /// Keep them off.
        #[arg(long)]
        deny: bool,
    },
    /// Check plugins: the manifest reads, and the code registers what it
    /// declares. Exits with an error if one does not. A file is a
    /// plugin's archive: checked as the installer checks it, and its code
    /// as it is in the archive.
    Check { folders: Vec<PathBuf> },
    /// Run a plugin's command on a module and save: its edits applied as
    /// one command, all or none. The plugin runs sandboxed, as in the
    /// window.
    Run {
        module: PathBuf,
        /// The plugin's folder.
        plugin: PathBuf,
        /// The command's id (`mg plugin list` shows them); with --event,
        /// the id of what the plugin does by itself.
        command: String,
        /// Run what the plugin does by itself when a module is opened or
        /// saved (its event of this id), as the window would then.
        #[arg(long)]
        event: bool,
        /// With --event: a resource that changed (a module saved), that a
        /// save writes (a resource saving or saved) or that was made
        /// (`guard.utc`), as often as needed. None named: all of them (a
        /// save's: those that differ from --was).
        #[arg(long, value_name = "NAME", requires = "event")]
        changed: Vec<String>,
        /// What the command is run on, as if selected in the window
        /// (`ctx.selection`), as often as needed: a blueprint or an area
        /// (`guard.utc`, `tavern.are`), or an object placed in an area as
        /// its GIT and its place there (`tavern.git/Placeable List[3]`).
        /// With --event, the objects placed or pasted.
        #[arg(long, value_name = "NAME[/PATH]")]
        select: Vec<String>,
        /// With --event, for a resource saving or saved: the module as it
        /// was before (its resources the old ones; those that differ are
        /// what the save writes). None: each resource as it is.
        #[arg(long, value_name = "MODULE", requires = "event")]
        was: Option<PathBuf>,
        /// With --event: what made the objects or resources
        /// (`ctx.event.by`: wizard, paste, palette…).
        #[arg(long, requires = "event")]
        by: Option<String>,
        /// What is open in front, as in the window (`ctx.open`): named as
        /// with --select, or any resource of the module (`old.nss`).
        #[arg(long, value_name = "NAME[/PATH]", conflicts_with = "event")]
        open: Option<String>,
        /// A form's field, `ID=VALUE` (the others take their defaults).
        #[arg(long)]
        answer: Vec<String>,
        /// A file or folder for the plugin where it asks for one, in the
        /// order it asks.
        #[arg(long)]
        file: Vec<PathBuf>,
        /// Where the haks it writes go (else the user folder's `hak`).
        #[arg(long)]
        hak_dir: Option<PathBuf>,
        /// Answer yes where the plugin asks to confirm, or to write a hak
        /// (else: no).
        #[arg(long)]
        yes: bool,
        /// A setting of the plugin for this run, `ID=VALUE` (repeatable),
        /// over what the project's files hold and the defaults. What the
        /// plugin sets itself is kept in the project's files (but for a
        /// setting of scope "user": mg keeps none).
        #[arg(long, value_name = "ID=VALUE")]
        setting: Vec<String>,
        /// Print the edits it would make; change nothing.
        #[arg(long)]
        dry_run: bool,
    },
    /// Run a plugin's own tests: the files `tests/*.toml` in its folder,
    /// each naming a fixture module and the checks, commands and events to
    /// run on it, with what is to come of each (the findings and their
    /// fixes, the fields after the edits, the log). Nothing is written to
    /// the fixture. Exits with an error if a test fails, or there are none.
    /// The format: the manual's Writing Plugins chapter, "A plugin's own
    /// tests".
    Test {
        /// The plugin's folder.
        plugin: PathBuf,
        /// The fixture module for every test (a module folder, a .mod or
        /// a nasher project), in place of those the test files name.
        #[arg(long)]
        module: Option<PathBuf>,
    },
    /// Pack a plugin's folder into an archive to hand around (a zip that
    /// Install Plugin from File and `mg plugin install` take). Hidden
    /// files are left out; the plugin is checked first.
    Pack {
        /// The plugin's folder.
        plugin: PathBuf,
        /// The archive to write (default: ID-VERSION.zip, here).
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Install a plugin from its archive into a folder of plugins, as
    /// Install Plugin from File does: the archive is checked, and none of
    /// the plugin runs.
    Install {
        /// The plugin's archive (a zip).
        file: PathBuf,
        /// The folder of plugins to install into.
        folder: PathBuf,
        /// Install over the same plugin installed from an archive before.
        #[arg(long)]
        replace: bool,
    },
    /// Remove a plugin that was installed from an archive from a folder of
    /// plugins. One put there by hand is left alone: delete its folder.
    Remove {
        /// The plugin's id (`mg plugin list` shows it).
        id: String,
        /// The folder of plugins.
        folder: PathBuf,
    },
}

/// `mg vars`: the project's described variables.
fn vars(cli: &Cli, path: &Path, on: Option<&str>, name: Option<&str>) -> Result<Output> {
    use mg_module::var_desc::{ModuleLookup, Owner, type_label};
    use serde_json::json;
    let m = Module::open(path)?;
    let owner = match on {
        Some(o) => Some(Owner::from_name(o).ok_or_else(|| {
            let all: Vec<&str> = Owner::ALL.iter().map(|o| o.name()).collect();
            anyhow::anyhow!("--on {o}: one of {}", all.join(", "))
        })?),
        None => None,
    };
    let p = &m.project_file;
    let mut out = Output::default();
    project_file_notes(&m, &mut out);
    if !p.variables_found {
        out.note(format!(
            "{} describes no variables: write a {} beside its moonglow.toml",
            path.display(),
            mg_module::var_desc::FILE_NAME
        ));
    }
    // (The game's tables and scripts, where a description names them.)
    let game = install(cli).ok().and_then(|gi| mg_rules::GameData::open(&gi).ok());
    let lookup = ModuleLookup::new(&m, game.as_ref().map(|g| &g.resman));
    let mut rows = Vec::new();
    for d in &p.variables.described {
        // (`--on`: what goes on one, as the Variables window offers it,
        // never one the scripts alone set.)
        if owner.is_some_and(|o| d.runtime || !d.belongs_on(o)) {
            continue;
        }
        if let Some(n) = name
            && !(d.name == n || d.matches(n))
        {
            continue;
        }
        let c = d.choices(&lookup);
        let values: Vec<serde_json::Value> =
            c.list.iter().map(|x| json!({ "value": x.value, "note": x.note })).collect();
        let mut line = format!("{}\t{}", d.name, d.words());
        if !c.list.is_empty() {
            let shown: Vec<String> = c
                .list
                .iter()
                .take(12)
                .map(|x| match &x.note {
                    Some(n) => format!("{} ({n})", x.value),
                    None => x.value.clone(),
                })
                .collect();
            let more = if c.list.len() > shown.len() { ", …" } else { "" };
            line.push_str(&format!("\t{}{more}", shown.join(", ")));
        }
        if let Some(h) = &d.help {
            line.push_str(&format!("\t{h}"));
        }
        out.line(line);
        if let Some(problem) = &c.problem {
            out.note(format!("{}: {problem}", d.name));
        }
        rows.push(json!({
            "name": d.name,
            "line": d.line,
            "type": d.var_type.map(type_label),
            "on": d.on.iter().map(|o| o.name()).collect::<Vec<_>>(),
            "min": d.min,
            "max": d.max,
            "other": d.other,
            "runtime": d.runtime,
            "values": values,
            "help": d.help,
        }));
    }
    out.json = json!({ "undescribed": p.variables.undescribed, "variables": rows });
    Ok(out)
}

/// What the module's project file (`moonglow.toml`) said that wasn't
/// understood, as notes.
fn project_file_notes(m: &Module, out: &mut Output) {
    let p = &m.project_file;
    for line in p.errors.iter().chain(&p.warnings) {
        out.note(line.clone());
    }
}

/// What a command produced: its result as JSON, and the same for people
/// as lines of standard output and notes (warnings, summaries) on
/// standard error.
#[derive(Default)]
struct Output {
    json: serde_json::Value,
    text: Vec<String>,
    notes: Vec<String>,
    /// Found errors (verify): printed, then a failing exit.
    failed: bool,
    /// Written already (cat's bytes, the language server).
    done: bool,
}

impl Output {
    fn new(json: serde_json::Value) -> Output {
        Output { json, ..Default::default() }
    }

    fn line(&mut self, s: impl Into<String>) {
        self.text.push(s.into());
    }

    fn note(&mut self, s: impl Into<String>) {
        self.notes.push(s.into());
    }

    /// Prints it as text, or as JSON with the notes in "notes".
    fn print(mut self, json: bool) -> io::Result<()> {
        if self.done {
            return Ok(());
        }
        let mut out = io::stdout().lock();
        if json {
            if let serde_json::Value::Object(o) = &mut self.json
                && !self.notes.is_empty()
            {
                o.insert("notes".into(), self.notes.into());
            }
            let text = serde_json::to_string_pretty(&self.json).map_err(io::Error::other)?;
            writeln!(out, "{text}")?;
        } else {
            for l in &self.text {
                writeln!(out, "{l}")?;
            }
            for n in &self.notes {
                eprintln!("{n}");
            }
        }
        Ok(())
    }
}

/// A file's extension, lower case.
fn extension(path: &Path) -> String {
    path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default()
}

/// The talk table in the file at `path`, with the feminine table beside
/// it (the same name with an `f` added) if there is one; a new, empty
/// one (English) where the file isn't there and `make` allows.
fn talk_table(path: &Path, make: bool) -> Result<mg_module::talk::Table> {
    use mg_module::talk::{Found, Source, Table, feminine};
    let name = path.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    if !path.is_file() && make {
        let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
        let mut t = Table::create(&name, &dir, mg_core::Language::ENGLISH, false);
        t.source = Source::File(path.to_path_buf());
        return Ok(t);
    }
    let found = |p: &Path| -> Result<Found> {
        let data = std::fs::read(p).with_context(|| p.display().to_string())?;
        Ok(Found { source: Source::File(p.to_path_buf()), data })
    };
    let beside = path.with_file_name(format!("{}.tlk", feminine(&name)));
    let fem = if beside.is_file() { Some(found(&beside)?) } else { None };
    Table::open(&name, found(path)?, fem).map_err(|e| anyhow::anyhow!("{e}"))
}

fn main() -> ExitCode {
    // (What the libraries write of the release reads it from there.)
    mg_core::release::set(env!("CARGO_PKG_VERSION"));
    let cli = Cli::parse();
    let json = cli.json;
    let result = run(&cli).and_then(|out| {
        let failed = out.failed;
        out.print(json)?;
        Ok(failed)
    });
    match result {
        Ok(false) => ExitCode::SUCCESS,
        Ok(true) => ExitCode::FAILURE,
        // The reader went away (`mg cat x | head`): not an error.
        Err(e)
            if e.downcast_ref::<io::Error>()
                .is_some_and(|e| e.kind() == io::ErrorKind::BrokenPipe) =>
        {
            ExitCode::SUCCESS
        }
        Err(e) => {
            if json {
                println!("{}", serde_json::json!({ "error": format!("{e:#}") }));
            } else {
                eprintln!("mg: {e:#}");
            }
            ExitCode::FAILURE
        }
    }
}

fn install(cli: &Cli) -> Result<GameInstall> {
    let mut gi = match &cli.root {
        Some(r) => GameInstall::new(r, None, "en"),
        None => {
            GameInstall::detect().context("no game install found; pass --root or set NWN_ROOT")?
        }
    };
    if !GameInstall::is_install(&gi.root) {
        bail!("{} is not a game install (no data/nwn_base.key)", gi.root.display());
    }
    if let Some(u) = &cli.user_dir {
        gi.user_dir = Some(u.clone());
    }
    if cli.no_user_dir {
        gi.user_dir = None;
    }
    Ok(gi)
}

/// The codepage the game client reads text in, as its `settings.tml`
/// (the user directory's) sets it: `game.language.codepage`, else the
/// codepage of `game.language.override`'s language. `None` where neither
/// is set (the game's language is then the install's).
fn client_codepage(settings: &str) -> Option<Codepage> {
    let table: toml::Table = settings.parse().ok()?;
    let language = table.get("game")?.get("language")?;
    let text = |key: &str| language.get(key)?.as_str().map(str::trim).filter(|s| !s.is_empty());
    if let Some(cp) = text("codepage").and_then(Codepage::for_label) {
        return Some(cp);
    }
    let code = text("override")?;
    let lang = mg_core::Language::EE.into_iter().find(|l| l.short_code() == Some(code))?;
    Some(lang.codepage())
}

fn resource_key(name: &str) -> Result<ResKey> {
    ResKey::from_filename(name)
        .with_context(|| format!("{name:?} is not a resource name like classes.2da"))
}

fn path_text(p: &Path) -> String {
    p.display().to_string()
}

/// Compile results: errors as lines and as JSON.
fn compiled(out: &mut Output, results: &[mg_module::build::ScriptResult]) -> serde_json::Value {
    let mut errors = Vec::new();
    for r in results {
        if let Err(e) = &r.result {
            out.line(e.message.clone());
            let (script, line) =
                e.location().map_or((r.script.resref.to_string(), None), |(s, l)| (s, Some(l)));
            errors
                .push(serde_json::json!({ "script": script, "line": line, "message": e.message }));
        }
    }
    serde_json::json!({ "scripts": results.len(), "failed": errors.len(), "errors": errors })
}

fn run(cli: &Cli) -> Result<Output> {
    use serde_json::json;
    let out = match &cli.cmd {
        Cmd::Ls { archive } => {
            let data = std::fs::read(archive).with_context(|| path_text(archive))?;
            let erf = Erf::read(&data)?;
            let mut out = Output::new(json!({
                "archive": path_text(archive),
                "type": String::from_utf8_lossy(&erf.file_type).trim(),
                "resources": erf.entries.iter().map(|e| json!({ "name": e.filename(), "size": e.size })).collect::<Vec<_>>(),
            }));
            for e in &erf.entries {
                out.line(format!("{:>10}  {}", e.size, e.filename()));
            }
            out
        }
        Cmd::Unpack { archive, out: dir } => {
            let data = std::fs::read(archive).with_context(|| path_text(archive))?;
            let erf = Erf::read(&data)?;
            std::fs::create_dir_all(dir)?;
            for e in &erf.entries {
                let path = ResKey::new(e.resref, e.restype).path_in(dir)?;
                std::fs::write(path, erf.data(e)?)?;
            }
            let mut out =
                Output::new(json!({ "folder": path_text(dir), "files": erf.entries.len() }));
            out.note(format!("unpacked {} files", erf.entries.len()));
            out
        }
        Cmd::Pack { dir, archive, compile_models } => {
            let ext =
                archive.extension().and_then(|e| e.to_str()).unwrap_or("erf").to_ascii_uppercase();
            let mut file_type = [b' '; 4];
            file_type[..ext.len().min(4)].copy_from_slice(&ext.as_bytes()[..ext.len().min(4)]);
            let mut w = ErfWriter::new(file_type);
            let mut paths: Vec<PathBuf> =
                std::fs::read_dir(dir)?.flatten().map(|e| e.path()).collect();
            paths.sort();
            // The models kept as text, compiled (asked for): their bytes
            // in place of the files'.
            let mut compiled: std::collections::HashMap<ResKey, Vec<u8>> = Default::default();
            let (mut made, mut left, mut said) = (Vec::new(), Vec::new(), Vec::new());
            if *compile_models {
                let mut models = Vec::new();
                for p in paths.iter().filter(|p| p.is_file()) {
                    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
                    // (Materials too: a mesh with no render hint takes its
                    // material's.)
                    if let Ok(key) = resource_key(name)
                        && matches!(key.restype, mg_core::ResType::MDL | mg_core::ResType::MTR)
                    {
                        models.push((key, std::fs::read(p)?));
                    }
                }
                let files: Vec<(ResKey, &[u8])> =
                    models.iter().map(|(k, d)| (*k, d.as_slice())).collect();
                let game = install(cli).ok().and_then(|gi| ResMan::for_game(&gi).ok());
                let lookup = |name: &str, t: mg_core::ResType| {
                    let rm = game.as_ref()?;
                    rm.get_named(name, t).ok().map(|d| d.into_owned())
                };
                for c in mg_module::models::compile_models(&files, &lookup) {
                    match c.result {
                        Ok((binary, notes)) => {
                            for (line, note) in notes {
                                let at =
                                    if line > 0 { format!(" line {line}") } else { String::new() };
                                said.push(format!("{}{at}: {note}", c.model));
                            }
                            made.push(c.model.to_string());
                            compiled.insert(c.model, binary);
                        }
                        Err(why) => left.push((c.model.to_string(), why)),
                    }
                }
                if game.is_none() && !left.is_empty() {
                    said.push(
                        "no game install found: supermodels were looked for in the directory only"
                            .into(),
                    );
                }
            }
            for p in paths.iter().filter(|p| p.is_file()) {
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
                let key = resource_key(name)?;
                let data = match compiled.remove(&key) {
                    Some(binary) => binary,
                    None => std::fs::read(p)?,
                };
                w.add(key.resref, key.restype, data)?;
            }
            // Streamed: a hak can be gigabytes.
            let mut file = io::BufWriter::new(std::fs::File::create(archive)?);
            w.write_to(&mut file)?;
            file.flush()?;
            let past = w.past_read_limit();
            let mut out = Output::new(json!({
                "archive": path_text(archive),
                "files": w.len(),
                "past_read_limit": past.iter().map(|(r, t)| format!("{r}.{t}")).collect::<Vec<_>>(),
                "models_compiled": made,
                "models_left_as_text": left
                    .iter()
                    .map(|(m, why)| json!({ "model": m, "why": why }))
                    .collect::<Vec<_>>(),
                "model_notes": said,
            }));
            out.note(format!("packed {} files", w.len()));
            if *compile_models {
                out.note(format!("compiled {} models kept as text", made.len()));
            }
            for (model, why) in &left {
                out.note(format!("warning: {model} goes in as text: {why}"));
            }
            for note in &said {
                out.note(note.clone());
            }
            if let Some(first) = past.first() {
                out.note(format!(
                    "warning: {} file(s) from {}.{} on start past 2 GiB into the archive, \
                     which the game can't read; split it into two",
                    past.len(),
                    first.0,
                    first.1
                ));
            }
            out
        }
        Cmd::Gff { input, output } => gff(input, output.as_deref(), cli.json)?,
        Cmd::Set { module, resource, fields, remove, dry_run, plugins } => {
            let m = Module::open(module)?;
            let codepage = module_codepage(cli, &m);
            let mut notes = Output::default();
            let guard = save_guard(cli, &m, plugins, &mut notes)?;
            let mut out =
                edits::set(m, resource, fields, remove, *dry_run, codepage, guard.as_ref())?;
            out.notes.splice(0..0, notes.notes);
            out
        }
        Cmd::Apply { module, edits, dry_run, plugins } => {
            let m = Module::open(module)?;
            let codepage = module_codepage(cli, &m);
            let mut notes = Output::default();
            let guard = save_guard(cli, &m, plugins, &mut notes)?;
            let mut out = edits::apply(m, edits, *dry_run, codepage, guard.as_ref())?;
            out.notes.splice(0..0, notes.notes);
            out
        }
        Cmd::Which { resource, module } => {
            let gi = install(cli)?;
            let key = resource_key(resource)?;
            let mut out = Output::default();
            let rm = match module {
                Some(path) => module_resman(&gi, &Module::open(path)?, &mut out)?,
                None => ResMan::for_game(&gi)?,
            };
            // Every copy, highest first: the first is the one the game reads.
            let copies = rm.copies(&key);
            let Some(&top) = copies.first() else { bail!("{key} not found") };
            let label = |i: usize| rm.layers()[i].label.as_str();
            out.json = json!({
                "resource": key.to_string(),
                "layer": label(top),
                "copies": copies.iter().map(|&i| json!({
                    "layer": label(i),
                    "priority": rm.layers()[i].priority,
                    "overridden_by": (i != top).then(|| label(top)),
                })).collect::<Vec<_>>(),
            });
            out.line(label(top));
            for &i in &copies[1..] {
                out.line(format!("{} (overridden by {})", label(i), label(top)));
            }
            out
        }
        Cmd::Cat { resource, text } => {
            let rm = ResMan::for_game(&install(cli)?)?;
            let key = resource_key(resource)?;
            let data = rm.get(&key)?;
            // (A model that is text already is printed as it is.)
            let data = match text.then(|| mg_mdl::decompile(&data)) {
                Some(Ok(text)) => text.into_bytes().into(),
                Some(Err(mg_mdl::DecompileError::NotBinary)) | None => data,
                Some(Err(e)) => anyhow::bail!("{key}: {e}"),
            };
            if cli.json {
                // Text as text; other bytes as hexadecimal.
                let text = std::str::from_utf8(&data).ok().map(str::to_string);
                let hex = text
                    .is_none()
                    .then(|| data.iter().map(|b| format!("{b:02x}")).collect::<String>());
                Output::new(json!({
                    "resource": key.to_string(),
                    "layer": rm.origin(&key),
                    "size": data.len(),
                    "text": text,
                    "hex": hex,
                }))
            } else {
                io::stdout().lock().write_all(&data)?;
                Output { done: true, ..Default::default() }
            }
        }
        Cmd::Layers => {
            let rm = ResMan::for_game(&install(cli)?)?;
            let mut out = Output::new(json!({
                "layers": rm.layers().iter().map(|l| json!({
                    "label": l.label,
                    "priority": l.priority,
                    "class": format!("{:?}", l.class),
                    "resources": l.container.len(),
                })).collect::<Vec<_>>(),
            }));
            for l in rm.layers() {
                out.line(format!(
                    "{:>3}  {:>7}  {:?}  {}",
                    l.priority,
                    l.container.len(),
                    l.class,
                    l.label
                ));
            }
            out
        }
        Cmd::Verify {
            module,
            unused,
            plugins,
            skip,
            fix,
            dry_run,
            allow_project_plugins,
            timings,
        } => {
            let fixes = Fixes { checks: fix, dry_run: *dry_run };
            let plugins = plugins::Chosen { dirs: plugins, project: *allow_project_plugins };
            let mut times = mg_module::doctor::Timings::default();
            let started = std::time::Instant::now();
            let out = verify(&install(cli)?, module, *unused, &plugins, skip, &fixes, &mut times);
            if *timings {
                times.add("in all", started.elapsed());
                for (part, took) in &times.parts {
                    eprintln!("{:>9.3} s  {part}", took.as_secs_f64());
                }
            }
            out?
        }
        Cmd::Plugin { cmd } => match cmd {
            PluginCmd::List { folders, project } => plugins::list(folders, project.as_deref())?,
            PluginCmd::Check { folders } => plugins::check(folders)?,
            PluginCmd::Allow { project, ids, deny } => plugins::allow(project, ids, *deny)?,
            // (A plugin that reads no game data needs no game.)
            PluginCmd::Test { plugin, module } => {
                plugins::test(install(cli).ok().as_ref(), plugin, module.as_deref())?
            }
            PluginCmd::Run {
                module,
                plugin,
                command,
                event,
                changed,
                select,
                was,
                by,
                open,
                answer,
                file,
                hak_dir,
                yes,
                setting,
                dry_run,
            } => {
                let host = plugins::CliHost::new(answer, file, *yes)?;
                // (A plugin that reads no game data needs no game.)
                let gi = install(cli).ok();
                let hak_dir = hak_dir.as_deref();
                let changed = match event {
                    true => Some(
                        changed
                            .iter()
                            .map(|n| {
                                ResKey::from_filename(n).ok_or_else(|| {
                                    anyhow::anyhow!("{n} is no resource's name (name.ext)")
                                })
                            })
                            .collect::<Result<Vec<_>>>()?,
                    ),
                    false => None,
                };
                let selected = |n: &String| {
                    mg_plugin::Selected::parse(n).map_err(|e| anyhow::anyhow!("--select {e}"))
                };
                let selection = mg_plugin::Selection {
                    selected: select.iter().map(selected).collect::<Result<_>>()?,
                    open: open.as_ref().map(selected).transpose()?,
                };
                let what = plugins::What {
                    id: command,
                    changed,
                    selection,
                    settings: setting,
                    was: was.as_deref(),
                    by: by.as_deref(),
                };
                plugins::run(gi.as_ref(), module, plugin, &what, host, hak_dir, *dry_run)?
            }
            PluginCmd::Pack { plugin, output } => plugins::pack(plugin, output.as_deref())?,
            PluginCmd::Install { file, folder, replace } => {
                plugins::install(file, folder, *replace)?
            }
            PluginCmd::Remove { id, folder } => plugins::remove(id, folder)?,
        },
        Cmd::Nui { cmd } => nui::run(cli, cmd)?,
        Cmd::Vars { module, on, name } => vars(cli, module, on.as_deref(), name.as_deref())?,
        Cmd::BuilderCode { project, code, clear } => {
            use mg_module::project_file::{DEFAULT_PATTERN, ProjectFile};
            let root = match project.file_name() {
                Some(n) if n == "nasher.cfg" => project.parent().unwrap_or(Path::new(".")),
                _ => project.as_path(),
            };
            if !root.is_dir() {
                bail!("{}: not a project's folder", root.display());
            }
            if code.is_some() || *clear {
                ProjectFile::set_builder_code(root, code.as_deref())?;
            }
            let p = ProjectFile::read(root);
            let pattern = p.resref_pattern.clone().unwrap_or_else(|| DEFAULT_PATTERN.into());
            let naming = p.naming();
            let mut out = Output::new(json!({
                "code": p.user.builder_code,
                "pattern": pattern,
                "example": (!naming.is_plain()).then(|| naming.fit("market", "")),
            }));
            match &p.user.builder_code {
                Some(c) => out.line(format!(
                    "{c}: new ResRefs are made by {pattern} (a new area named Market: {})",
                    naming.fit("market", "")
                )),
                None => out.line("no builder code: new ResRefs are made from names alone"),
            }
            for line in p.errors.iter().chain(&p.warnings) {
                out.note(line.clone());
            }
            out
        }
        Cmd::Claims { project, action, areas, force } => {
            claims::run(project, *action, areas, *force)?
        }
        Cmd::Checks => {
            use mg_module::doctor::Check;
            let mut out = Output::new(json!({
                "checks": Check::ALL
                    .iter()
                    .map(|c| json!({ "id": c.id(), "group": c.group().id(), "about": c.about() }))
                    .collect::<Vec<_>>(),
            }));
            for c in Check::ALL {
                out.line(format!("{}\t{}\t{}", c.id(), c.group().id(), c.about()));
            }
            out
        }
        Cmd::Haks { module, usage } => {
            let m = Module::open(module)?;
            let gi = install(cli)?;
            let report = mg_module::haks::hak_report(&gi, &m.haks()?)?;
            let rows = |it: &mut dyn Iterator<Item = (&ResKey, &Vec<String>)>| {
                it.map(|(k, h)| json!({ "resource": k.to_string(), "haks": h })).collect::<Vec<_>>()
            };
            let mut out = Output::new(json!({
                "haks": report.haks.iter().map(|(n, p)| json!({ "name": n, "path": p.as_deref().map(path_text) })).collect::<Vec<_>>(),
                "conflicts": rows(&mut report.conflicts()),
                "overrides": rows(&mut report.overrides.iter()),
                "resources": report.resources.len(),
            }));
            if *usage {
                let started = std::time::Instant::now();
                let (haks, reached) = usage::hak_usage(&gi, &m, &report)?;
                for u in &haks {
                    out.line(usage::line(u));
                }
                let (used, all): (usize, usize) =
                    haks.iter().fold((0, 0), |(a, b), u| (a + u.used.len(), b + u.total()));
                let (used_bytes, bytes) =
                    haks.iter().fold((0, 0), |(a, b), u| (a + u.used_bytes(), b + u.bytes()));
                let none: Vec<&usage::HakUsage> =
                    haks.iter().filter(|u| u.path.is_some() && u.used.is_empty()).collect();
                let kept = none.iter().filter(|u| !u.not_judged.is_empty()).count();
                out.line(format!(
                    "All\t{used} of {all} used ({} of {}); nothing reaches {} of {} haks ({kept} \
                     of them hold what isn't judged, which the game may read itself)",
                    usage::size(used_bytes),
                    usage::size(bytes),
                    none.len(),
                    haks.len()
                ));
                out.json["usage"] = usage::json(&haks);
                out.json["reached"] = json!(reached);
                out.note(format!(
                    "the walk reached {reached} resources in {:.1} s; not followed: what scripts \
                     reach by number (effects, spells, appearances), what players choose, and \
                     what the game reads itself (\"not judged\")",
                    started.elapsed().as_secs_f64()
                ));
            } else {
                out.line(report.to_text().trim_end());
            }
            out
        }
        Cmd::Export { module, resources, output, files, comment, keep_factions } => {
            let m = Module::open(module)?;
            let mut out = Output::default();
            let roots = resources.iter().map(|r| resource_key(r)).collect::<Result<Vec<_>>>()?;
            for r in &roots {
                if !m.contains(r) {
                    bail!("{r} is not in the module");
                }
            }
            if *files {
                // (No game data needed: nothing is looked up.)
                let mut set: Vec<ResKey> = Vec::new();
                for r in &roots {
                    for k in mg_module::transfer::file_set(&m, r) {
                        if !set.contains(&k) {
                            set.push(k);
                        }
                    }
                }
                for k in set.iter().filter(|k| k.restype == mg_core::ResType::NSS) {
                    if !set.contains(&ResKey::new(k.resref, mg_core::ResType::NCS)) {
                        out.note(format!("warning: {k} isn't compiled: no .ncs written"));
                    }
                }
                let written = mg_module::transfer::export_files(&m, &set, output)?;
                for p in &written {
                    out.line(path_text(p));
                }
                out.json = json!({
                    "folder": path_text(output),
                    "files": written.iter().map(|p| path_text(p)).collect::<Vec<_>>(),
                    "resources": set.iter().map(ToString::to_string).collect::<Vec<_>>(),
                });
                out.note(format!("wrote {} files", written.len()));
                return Ok(out);
            }
            let rm = module_resman(&install(cli)?, &m, &mut out)?;
            let plan = mg_module::transfer::plan_export(&m, &roots, &rm);
            for r in &plan.missing {
                out.note(format!(
                    "warning: {} {} needs {:?} {}, found nowhere",
                    r.from, r.path, r.kind, r.target
                ));
            }
            // (Where it came from, after the description: Import says
            // when the module has moved on since.)
            let stamp = mg_module::stamp::Stamp::of(&m);
            let description = mg_module::stamp::describe(comment, &stamp);
            let erf =
                mg_module::transfer::export_erf(&m, &plan.resources, &description, !keep_factions)?;
            std::fs::write(output, erf)?;
            out.json = json!({
                "erf": path_text(output),
                "resources": plan.resources.iter().map(ToString::to_string).collect::<Vec<_>>(),
                "stamp": stamp_json(&stamp),
            });
            out.note(format!(
                "exported {} resources, stamped with {}",
                plan.resources.len(),
                stamp.summary()
            ));
            out
        }
        Cmd::Compile { module, uncompiled, all, ndb } => {
            let mut m = Module::open(module)?;
            let mut out = Output::default();
            let gi = install(cli)?;
            let rm = module_resman(&gi, &m, &mut out)?;
            use mg_module::build;
            if *uncompiled {
                let names: Vec<ResKey> = (m.keys_of(ResType::NSS))
                    .filter(|k| !m.contains(&ResKey::new(k.resref, ResType::NCS)))
                    .filter(|k| {
                        let target = m.project.as_ref().map(|p| p.target());
                        !target.is_some_and(|t| t.skips_compiling(&k.to_string()))
                    })
                    .copied()
                    .collect();
                let results = build::compile_named_with(&mut m, &rm, &names, None, *ndb);
                out.json = compiled(&mut out, &results);
                m.save()?;
                let failed = out.json["failed"].as_u64().unwrap_or(0) as usize;
                out.note(format!("compiled {} scripts, {failed} failed", results.len() - failed));
                return Ok(out);
            }
            // What was compiled last, here or in the window, is passed
            // over: both keep it in one place.
            let dir = build::compiled_dir();
            let made_with = build::made_with(Some(module), None, Some(&gi.root), &rm);
            let known = match all {
                true => build::Remembered { made_with, ..Default::default() },
                false => build::Remembered::kept(dir.as_deref(), Some(module), made_with),
            };
            let pass = build::compile_changed(&mut m, &rm, None, &known, None, *ndb);
            out.json = compiled(&mut out, &pass.results);
            m.save()?;
            if let Some(dir) = &dir
                && let Err(e) = pass.now.write(&build::Remembered::file(dir, module))
            {
                out.note(format!("warning: what was compiled is not remembered: {e}"));
            }
            let failed = out.json["failed"].as_u64().unwrap_or(0) as usize;
            let unchanged = pass.scripts.saturating_sub(pass.results.len());
            out.json["unchanged"] = json!(unchanged);
            let left = match unchanged {
                0 => String::new(),
                n => format!("; {n} as they were when last compiled, left alone"),
            };
            out.note(format!(
                "compiled {} scripts, {failed} failed{left}",
                pass.results.len() - failed
            ));
            out
        }
        Cmd::Serve {
            module,
            dir,
            real_user_dir,
            port,
            public,
            name,
            player_password,
            dm_password,
            admin_password,
            env,
            preload,
            marker,
            until,
            timeout,
            restart_on_change,
            server_args,
        } => {
            use std::io::IsTerminal;
            let gi = install(cli)?;
            let config = serve::Config {
                module: module.clone(),
                dir: dir.clone(),
                real_user_dir: *real_user_dir,
                port: *port,
                public: *public,
                name: name.clone(),
                player_password: player_password.clone(),
                dm_password: dm_password.clone(),
                admin_password: admin_password.clone(),
                env: env.clone(),
                preload: preload.clone(),
                restart_on_change: *restart_on_change,
                stdin: true,
                args: server_args.clone(),
            };
            let started = std::time::Instant::now();
            let bold = !cli.json && io::stdout().is_terminal();
            let (mut reached, mut late) = (false, false);
            let end = serve::serve(&gi, &config, &mut |e| {
                let line = match e {
                    serve::Event::Log(line) => Some(line),
                    _ => None,
                };
                let marked = line.and_then(|l| marker.iter().find(|m| l.contains(m.as_str())));
                let mut out = io::stdout().lock();
                let _ = match (cli.json, serve::json(e), serve::text(e)) {
                    (true, Some(mut v), _) => {
                        if let Some(m) = marked {
                            v["event"] = json!("marker");
                            v["marker"] = json!(m);
                        }
                        writeln!(out, "{v}")
                    }
                    (false, _, Some(t)) if marked.is_some() && bold => {
                        writeln!(out, "\x1b[1m{t}\x1b[0m")
                    }
                    (false, _, Some(t)) => writeln!(out, "{t}"),
                    _ => Ok(()),
                };
                let _ = out.flush();
                if line.is_some_and(|l| until.as_ref().is_some_and(|u| l.contains(u.as_str()))) {
                    reached = true;
                    return serve::Flow::Stop;
                }
                if timeout.is_some_and(|t| started.elapsed().as_secs_f64() > t) {
                    late = true;
                    return serve::Flow::Stop;
                }
                serve::Flow::Continue
            })?;
            let failed = match end {
                serve::End::Stopped => late && until.is_some() && !reached,
                serve::End::NotLoaded => true,
                serve::End::Exited(code) => code != Some(0) || until.is_some(),
            };
            let said = match end {
                serve::End::Stopped if late && !reached => "stopped at the timeout".to_string(),
                serve::End::Stopped => "stopped".to_string(),
                serve::End::NotLoaded => "the server could not load the module".to_string(),
                serve::End::Exited(code) => format!("the server ended ({code:?})"),
            };
            if cli.json {
                println!("{}", json!({ "event": "ended", "how": said, "failed": failed }));
            } else {
                println!("mg: {said}");
            }
            Output { done: true, failed, ..Default::default() }
        }
        Cmd::Test {
            module,
            filter,
            tests,
            driver,
            settle,
            timeout,
            junit,
            log,
            include,
            dir,
            env,
            preload,
            server_args,
        } => {
            if *include {
                print!("{}", mg_module::testing::INCLUDE);
                return Ok(Output { done: true, ..Default::default() });
            }
            let Some(module) = module else { bail!("which module? (mg test MODULE)") };
            let gi = install(cli)?;
            let config = test::Config {
                module: module.clone(),
                filter: filter.clone(),
                tests: tests.clone(),
                own_driver: driver == "none",
                settle: *settle,
                timeout: *timeout,
                serve: serve::Config {
                    dir: dir.clone(),
                    name: "Moonglow test run".into(),
                    env: env.clone(),
                    preload: preload.clone(),
                    args: server_args.clone(),
                    ..Default::default()
                },
            };
            let resman = |m: &Module, notes: &mut Vec<String>| {
                let mut out = Output::default();
                let rm = module_resman(&gi, m, &mut out)?;
                notes.append(&mut out.notes);
                Ok(rm)
            };
            let ran = test::run(&gi, &config, resman, &mut |line| {
                if *log {
                    eprintln!("{line}");
                }
            })?;
            let mut out = Output::new(ran.report.to_json());
            out.json["module"] = json!(ran.name);
            out.json["seconds"] = json!(ran.seconds);
            out.json["compile_errors"] = json!(ran.compile_errors);
            out.json["ended"] = json!(ran.ended);
            out.notes = ran.notes;
            for e in &ran.compile_errors {
                out.line(e.clone());
            }
            for t in &ran.report.tests {
                match (&t.error, t.failed()) {
                    // (Run to its end with nothing asserted: said, not a
                    // failure.)
                    (None, 0) if t.cases.is_empty() => {
                        out.line(format!("ok    {} (0: it made no assertion)", t.script));
                    }
                    (None, 0) => out.line(format!("ok    {} ({})", t.script, t.cases.len())),
                    (None, n) => out.line(format!("FAIL  {} ({n} of {})", t.script, t.cases.len())),
                    (Some(_), _) => out.line(format!("ERROR {}", t.script)),
                }
                for c in t.cases.iter().filter(|c| !c.ok) {
                    let detail = match c.detail.is_empty() {
                        true => String::new(),
                        false => format!(": {}", c.detail),
                    };
                    out.line(format!("        {}{detail}", c.name));
                }
                for line in t.error.iter().flat_map(|e| e.lines()) {
                    out.line(format!("        {line}"));
                }
            }
            if let Some(file) = junit {
                std::fs::write(file, ran.report.junit(&ran.name, ran.seconds))
                    .with_context(|| path_text(file))?;
            }
            let ok = ran.compile_errors.is_empty() && ran.report.ok();
            out.json["ok"] = json!(ok);
            out.failed = !ok;
            let why = match (&ran.ended, ran.compile_errors.len()) {
                (_, n) if n > 0 => format!("{n} scripts did not compile; nothing was run"),
                (Some(why), _) => why.clone(),
                _ => format!("{:.1} s", ran.seconds),
            };
            out.note(format!(
                "{} passed, {} failed, {} errors in {} tests ({why})",
                ran.report.passed(),
                ran.report.failed(),
                ran.report.errors(),
                ran.report.tests.len()
            ));
            out
        }
        Cmd::TwodaCheck { paths, beside } => {
            // (A table that names nothing of the game's needs no game.)
            let game = install(cli).ok().and_then(|gi| mg_rules::GameData::open(&gi).ok());
            diff::check(paths, beside.as_deref(), game.as_ref(), Codepage::default())?
        }
        Cmd::TwodaDiff { a, b, base, output, edits, prefer, resource } => diff::twoda(
            &diff::Args {
                a,
                b,
                base: base.as_ref(),
                output: output.as_ref(),
                changes: edits.as_ref(),
                prefer: prefer.as_deref(),
                resource: resource.as_deref(),
            },
            Codepage::default(),
        )?,
        Cmd::TlkDiff { a, b, base, output, lines, prefer } => diff::tlk(
            &diff::Args {
                a,
                b,
                base: base.as_ref(),
                output: output.as_ref(),
                changes: lines.as_ref(),
                prefer: prefer.as_deref(),
                resource: None,
            },
            Codepage::default(),
        )?,
        Cmd::MergeGff { base, ours, theirs, prefer, output, path } => {
            merge::merge_gff(&merge::Args {
                base,
                ours,
                theirs,
                prefer: prefer.as_deref(),
                output: output.as_deref(),
                path: path.as_deref(),
            })?
        }
        Cmd::TlkAlloc { table, from, map, constants, prefix, start, check, against } => {
            tlk_alloc::run(&tlk_alloc::Args {
                table,
                from,
                map: map.as_ref(),
                constants: constants.as_ref(),
                prefix,
                start: *start,
                check: *check,
                against: against.as_ref(),
            })?
        }
        Cmd::Watch {
            project,
            dev_dir,
            r#mod,
            ndb,
            serve,
            dir,
            port,
            env,
            preload,
            server_args,
        } => {
            let gi = install(cli)?;
            let name = serve::module_name(project);
            let server_dir = serve.then(|| {
                let dir = dir.clone().unwrap_or_else(|| serve::scratch_dir(project));
                std::path::absolute(&dir).unwrap_or(dir)
            });
            let dev_dir = match (dev_dir, &server_dir) {
                (Some(dir), _) => dir.clone(),
                (None, Some(server)) => server.join("development"),
                (None, None) => bail!("where to? --dev-dir FOLDER, or --serve"),
            };
            std::fs::create_dir_all(&dev_dir).with_context(|| path_text(&dev_dir))?;
            let served = server_dir.as_ref().map(|d| d.join("modules").join(format!("{name}.mod")));
            let build_to = served.clone().or(r#mod.clone());
            let say = |v: serde_json::Value, lines: Vec<String>| {
                let mut out = io::stdout().lock();
                let _ = match cli.json {
                    true => writeln!(out, "{v}"),
                    false => lines.iter().try_for_each(|l| writeln!(out, "{l}")),
                };
                let _ = out.flush();
            };
            let resman = |m: &Module| -> Result<ResMan> {
                let mut notes = Output::default();
                let rm = module_resman(&gi, m, &mut notes)?;
                for note in notes.notes {
                    say(json!({ "event": "note", "note": note }), vec![format!("mg: {note}")]);
                }
                Ok(rm)
            };
            let also = |w: &watch::Watcher| -> Result<()> {
                // (The module built for the server, also where it was asked for.)
                if let (Some(to), Some(served)) = (r#mod, &served) {
                    std::fs::copy(served, to).with_context(|| path_text(to))?;
                }
                let _ = w;
                Ok(())
            };
            let mut rm = resman(&Module::open(project)?)?;
            let (mut w, first) = watch::Watcher::start(project, &dev_dir, build_to, *ndb, &rm)?;
            also(&w)?;
            say(
                json!({
                    "event": "watching", "project": path_text(project),
                    "dev_dir": path_text(&dev_dir), "compiled": first.compiled,
                    "errors": first.errors,
                }),
                [first.text(&dev_dir), vec![format!("mg: watching {}", project.display())]]
                    .concat(),
            );
            let Some(server_dir) = server_dir else {
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    if let Some(step) = w.look(&rm)? {
                        if !step.rebuild.is_empty() {
                            rm = resman(w.module())?;
                            also(&w)?;
                        }
                        say(step.json(), step.text(&dev_dir));
                    }
                }
            };
            let config = serve::Config {
                module: served.clone().expect("with --serve"),
                dir: Some(server_dir),
                port: *port,
                name: "Moonglow test server".into(),
                env: env.clone(),
                preload: preload.clone(),
                stdin: true,
                args: server_args.clone(),
                ..Default::default()
            };
            let mut wrong: Option<anyhow::Error> = None;
            let end = serve::serve(&gi, &config, &mut |e| {
                if let (Some(v), Some(t)) = (serve::json(e), serve::text(e)) {
                    say(v, vec![t]);
                }
                if *e != serve::Event::Idle {
                    return serve::Flow::Continue;
                }
                let step = match w.look(&rm) {
                    Ok(Some(step)) => step,
                    Ok(None) => return serve::Flow::Continue,
                    Err(e) => {
                        wrong = Some(e);
                        return serve::Flow::Stop;
                    }
                };
                say(step.json(), step.text(&dev_dir));
                if step.rebuild.is_empty() {
                    return serve::Flow::Continue;
                }
                match resman(w.module()).and_then(|new| also(&w).map(|()| new)) {
                    Ok(new) => rm = new,
                    Err(e) => {
                        wrong = Some(e);
                        return serve::Flow::Stop;
                    }
                }
                serve::Flow::Restart
            })?;
            if let Some(e) = wrong {
                return Err(e);
            }
            let failed = !matches!(end, serve::End::Stopped);
            say(json!({ "event": "ended", "failed": failed }), vec!["mg: stopped".into()]);
            Output { done: true, failed, ..Default::default() }
        }
        Cmd::Lsp { codepage } => {
            if cli.json {
                bail!("mg lsp speaks the Language Server Protocol, which is JSON already");
            }
            let codepage = match codepage {
                Some(name) => Codepage::for_label(name)
                    .with_context(|| format!("{name:?} isn't a codepage (cp1252, cp1250…)"))?,
                // (The user directory as `mg` finds it, without the game
                // too: the language server works without one.)
                None => (cli.user_dir.clone())
                    .or_else(|| {
                        let env = std::env::var_os("NWN_HOME")
                            .or_else(|| std::env::var_os("NWN_USER_DIRECTORY"));
                        env.map(PathBuf::from).or_else(mg_resman::default_user_dir)
                    })
                    .filter(|_| !cli.no_user_dir)
                    .and_then(|u| std::fs::read_to_string(u.join("settings.tml")).ok())
                    .and_then(|t| client_codepage(&t))
                    .unwrap_or(Codepage::WINDOWS_1252),
            };
            // The game's scripts and nwscript.nss, if there is a game.
            let game = match install(cli) {
                Ok(gi) => ResMan::for_game(&gi).ok(),
                Err(e) => {
                    eprintln!("mg lsp: {e:#}; only the workspace's scripts are known");
                    None
                }
            };
            lsp::serve(game, codepage)?;
            Output { done: true, ..Default::default() }
        }
        Cmd::Refs { module, name, tag } => {
            let m = Module::open(module)?;
            let (usages, mentions) = if *tag {
                (
                    mg_module::rename::tag_usages(&m, name),
                    mg_module::rename::mentions(&m, name, false),
                )
            } else {
                let key = ResKey::from_filename(name).context("give the resource as name.ext")?;
                (
                    mg_module::rename::usages(&m, key),
                    mg_module::rename::mentions(&m, &key.resref.to_string(), true),
                )
            };
            let mut out = Output::new(json!({
                "uses": usages.iter().map(|u| json!({ "place": u.place, "from": u.from.to_string(), "path": u.path })).collect::<Vec<_>>(),
                "script_strings": mentions.iter().map(|x| json!({ "script": x.script.to_string(), "line": x.line, "text": x.text })).collect::<Vec<_>>(),
            }));
            for u in &usages {
                out.line(format!("{}\t{} {}", u.place, u.from, u.path));
            }
            for x in &mentions {
                out.line(format!("{} line {}: {}", x.script, x.line, x.text));
            }
            out.note(format!("{} uses, {} script strings", usages.len(), mentions.len()));
            out
        }
        Cmd::Rename { module, from, to, strings } => {
            let mut m = Module::open(module)?;
            let from = ResKey::from_filename(from).context("give the resource as name.ext")?;
            let to = mg_core::ResRef::from_str(to).map_err(|e| anyhow::anyhow!("{to}: {e}"))?;
            let report = mg_module::rename::rename(&mut m, from, to, *strings)?;
            let mut out = Output::default();
            let mut compile = json!(null);
            if !report.recompile.is_empty() {
                let rm = module_resman(&install(cli)?, &m, &mut out)?;
                let results = mg_module::rename::recompile(&mut m, &rm, &report, None, false);
                compile = compiled(&mut out, &results);
            }
            m.save()?;
            let left = mg_module::rename::mentions(&m, &from.resref.to_string(), true);
            for x in &left {
                out.note(format!(
                    "warning: {} line {} still spells {}: {}",
                    x.script, x.line, from.resref, x.text
                ));
            }
            out.json = json!({
                "from": from.to_string(),
                "to": to.to_string(),
                "references": report.references,
                "in_scripts": report.in_scripts,
                "changed": report.changed.iter().map(ToString::to_string).collect::<Vec<_>>(),
                "compiled": compile,
            });
            out.note(format!(
                "renamed {from} to {to}: {} references, {} in scripts, {} scripts compiled again",
                report.references,
                report.in_scripts,
                report.recompile.len()
            ));
            out
        }
        Cmd::DialogExport { module, dialog, output } => {
            use mg_module::dialog_io::Format;
            let m = Module::open(module)?;
            let key = ResKey::from_filename(dialog).context("give the conversation as name.dlg")?;
            let g = m.gff(&key).with_context(|| format!("{key} is not in the module"))??;
            let f = Format::of(output).context("the output must end .txt, .csv, .twee or .ink")?;
            // (Its lines as the module's own table reads them, if any.)
            let text = f.write(&g, &key.resref.to_string());
            let text = Codepage::WINDOWS_1252.spelled_in(&text, module_codepage(cli, &m));
            std::fs::write(output, text.as_bytes())?;
            Output::new(
                json!({ "conversation": key.to_string(), "file": path_text(output), "format": f.name() }),
            )
        }
        Cmd::TlkExport { table, output, base } => {
            let t = talk_table(table, false)?;
            let (text, format) = match extension(output).as_str() {
                "json" => (t.to_json(), "json"),
                "csv" => (t.to_csv_as(!*base).into_bytes(), "csv"),
                _ => anyhow::bail!("the output must end .json or .csv"),
            };
            std::fs::write(output, text)?;
            Output::new(json!({
                "table": path_text(table),
                "file": path_text(output),
                "format": format,
                "lines": t.len(),
                "feminine": t.feminine.is_some(),
                "strrefs": if format == "csv" && !*base { "custom" } else { "line" },
            }))
        }
        Cmd::TlkImport { table, file, dry_run } => {
            let mut t = talk_table(table, true)?;
            let text = std::fs::read_to_string(file)?;
            let (changed, added) = match extension(file).as_str() {
                "json" => t.import_json(&text),
                "csv" => t.import_csv(&text),
                _ => anyhow::bail!("the file must end .json or .csv"),
            }
            .map_err(|e| anyhow::anyhow!("{}: {e}", file.display()))?;
            if !dry_run && (changed > 0 || added > 0 || !table.is_file()) {
                t.save().map_err(|e| anyhow::anyhow!("{e}"))?;
            }
            let mut out = Output::new(json!({
                "table": path_text(table),
                "changed": changed,
                "added": added,
                "lines": t.len(),
                "saved": !dry_run,
            }));
            out.line(format!(
                "{changed} lines changed, {added} added{}",
                if *dry_run { " (nothing written)" } else { "" }
            ));
            out
        }
        Cmd::DialogImport { module, file, name } => {
            use mg_module::dialog_io::Format;
            let mut m = Module::open(module)?;
            let stem = file.file_stem().map(|s| s.to_string_lossy().to_ascii_lowercase());
            let name = name.clone().or(stem).context("name the conversation with --name")?;
            let key = ResKey::parse(&name, mg_core::ResType::DLG)
                .with_context(|| format!("{name:?} isn't a resource name"))?;
            let source = std::fs::read_to_string(file)?;
            let source =
                (module_codepage(cli, &m).spelled_in(&source, Codepage::WINDOWS_1252)).into_owned();
            let mut out = Output::default();
            let mut changed = None;
            // A story is a conversation of its own; a text export goes
            // back into the conversation it came from.
            let format = Format::of(file);
            let g = if let Some(read) = format.and_then(|f| f.read(&source)) {
                read.map_err(|e| anyhow::anyhow!("{e}"))?
            } else if let Some(format) =
                format.filter(|f| f.reads() == mg_module::dialog_io::Reads::Text)
            {
                let mut g =
                    m.gff(&key).with_context(|| format!("{key} is not in the module"))??;
                let n = format
                    .update(&mut g, &source)
                    .expect("it reads text")
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                out.note(format!("{n} lines changed"));
                changed = Some(n);
                g
            } else {
                bail!("{}: not a .twee, .ink or .csv file", file.display())
            };
            m.set_gff(key, &g)?;
            m.save()?;
            let lines = mg_module::dialog::nodes(&g, mg_module::dialog::Kind::Entry).len()
                + mg_module::dialog::nodes(&g, mg_module::dialog::Kind::Reply).len();
            out.json =
                json!({ "conversation": key.to_string(), "lines": lines, "changed": changed });
            out.note(format!("{key}: {lines} lines"));
            out
        }
        Cmd::Replace { module, find, with, match_case, whole_word, only, dry_run } => {
            use mg_module::text::{Options, TextKind};
            let kinds: Vec<TextKind> = if only.is_empty() {
                TextKind::ALL.to_vec()
            } else {
                only.iter()
                    .map(|o| {
                        TextKind::ALL
                            .into_iter()
                            .find(|k| k.label().to_lowercase().starts_with(&o.to_lowercase()))
                            .with_context(|| {
                                format!(
                                    "{o}: not names, descriptions, conversations, journal or other"
                                )
                            })
                    })
                    .collect::<Result<_>>()?
            };
            let mut m = Module::open(module)?;
            let game = Some(module_codepage(cli, &m));
            let o = Options { match_case: *match_case, whole_word: *whole_word, game };
            let hits = mg_module::text::find(&m, find, o, &kinds);
            let mut out = Output::default();
            for h in &hits {
                out.line(format!("{}\t{}", h.place, h.text));
            }
            let times: usize = hits.iter().map(|h| h.count).sum();
            let mut replaced = 0;
            if *dry_run || hits.is_empty() {
                out.note(format!("{} strings, {times} times", hits.len()));
            } else {
                let (n, errors) = mg_module::text::replace(&mut m, &hits, find, with, o);
                for e in &errors {
                    out.note(format!("warning: {e}"));
                }
                m.save()?;
                replaced = n;
                out.note(format!("replaced {n} times in {} strings", hits.len() - errors.len()));
            }
            out.json = json!({
                "strings": hits.iter().map(|h| json!({ "place": h.place, "resource": h.key.to_string(), "kind": h.kind.label(), "text": h.text, "times": h.count })).collect::<Vec<_>>(),
                "times": times,
                "replaced": replaced,
                "dry_run": *dry_run,
            });
            out
        }
        Cmd::Areas {
            module,
            areas,
            text,
            tileset,
            interior,
            exterior,
            underground,
            above_ground,
            natural,
            artificial,
            sets,
            vars,
            remove_var,
            static_placeables,
            dynamic_placeables,
            grass,
            remove_grass,
            tileset_grass,
            lighting_scheme,
            all,
            dry_run,
        } => {
            use mg_module::areas::{self, Change, Filter};
            let resref =
                |s: &String| mg_core::ResRef::from_str(s).map_err(|e| anyhow::anyhow!("{s}: {e}"));
            // `--interior` or `--exterior`: so, not so, or either.
            let kind = |yes: bool, no: bool| if yes { Some(true) } else { no.then_some(false) };
            let filter = Filter {
                only: areas.iter().map(resref).collect::<Result<_>>()?,
                text: text.clone(),
                tileset: tileset.as_ref().map(resref).transpose()?,
                interior: kind(*interior, *exterior),
                underground: kind(*underground, *above_ground),
                natural: kind(*natural, *artificial),
            };
            let mut changes: Vec<Change> = Vec::new();
            for s in sets {
                changes.push(Change::parse_set(s).map_err(anyhow::Error::msg)?);
            }
            for v in vars {
                changes.push(Change::parse_var(v).map_err(anyhow::Error::msg)?);
            }
            changes.extend(remove_var.iter().cloned().map(Change::RemoveVar));
            if *static_placeables {
                changes.push(Change::StaticPlaceables);
            }
            if *dynamic_placeables {
                changes.push(Change::DynamicPlaceables);
            }
            // A surface material by surfacemat.2da's label (read only when a
            // material is named rather than numbered).
            let named = std::cell::OnceCell::new();
            let material = |name: &str| -> Option<u32> {
                let table = named.get_or_init(|| {
                    install(cli)
                        .ok()
                        .and_then(|gi| mg_rules::GameData::open(&gi).ok())
                        .and_then(|g| g.table("surfacemat").ok())
                });
                let t = table.as_ref()?;
                (0..t.len())
                    .find(|&r| t.get(r, "Label").is_some_and(|l| l.eq_ignore_ascii_case(name)))
                    .map(|r| r as u32)
            };
            for g in grass {
                let o = mg_module::grass::GrassOverride::parse(g, material)
                    .map_err(anyhow::Error::msg)?;
                changes.push(Change::Grass(o));
            }
            for name in remove_grass {
                let m = name.trim().parse().ok().or_else(|| material(name.trim()));
                let m = m.ok_or_else(|| anyhow::anyhow!("{name}: not a surface material"))?;
                changes.push(Change::RemoveGrass(m));
            }
            if let Some(on) = tileset_grass {
                let on = areas::yes_no(on)
                    .ok_or_else(|| anyhow::anyhow!("--tileset-grass {on:?}: yes or no"))?;
                changes.push(Change::TilesetGrass(on));
            }
            let mut m = Module::open(module)?;
            let mut out = Output::default();
            // A lighting scheme, from the game's environment.2da (the
            // module's haks' over it): before the fields `--set` sets.
            if let Some(text) = lighting_scheme {
                install(cli).context("--lighting-scheme reads the game's environment.2da")?;
                let game = search::game_data(cli, &m, &mut out)
                    .context("--lighting-scheme: the game's data can't be read")?;
                let row = mg_module::new::Scheme::find(&game, text)
                    .map_err(|e| anyhow::anyhow!("--lighting-scheme {e}"))?;
                let scheme = mg_module::new::Scheme::read(&game, row)?;
                changes.insert(0, Change::LightingScheme(scheme));
            }
            // A variable's type, where none is given: its description's
            // (`variables.toml`), where the project describes it.
            let mut described_notes = Vec::new();
            for c in &mut changes {
                let Change::Var { name, kind, .. } = c else { continue };
                let Some(d) = m.project_file.variables.describe(name) else { continue };
                let file = mg_module::var_desc::FILE_NAME;
                if d.runtime {
                    described_notes.push(format!(
                        "{name}: {file}:{} says the scripts set it while the server runs \
                         (runtime), never in a file: set here, it is there from the start",
                        d.line
                    ));
                }
                if !d.belongs_on(mg_module::var_desc::Owner::Area) {
                    described_notes.push(format!(
                        "{name}: {file}:{} says it goes on {}, not on an area",
                        d.line,
                        d.on.iter().map(|o| o.name()).collect::<Vec<_>>().join(", ")
                    ));
                }
                let Some(t) = d.var_type.filter(|t| t.field_type().is_some()) else { continue };
                match *kind {
                    None => *kind = Some(t.code()),
                    Some(k) if k != t.code() => described_notes.push(format!(
                        "{name}: set as {} as given, and {file}:{} says it is {}",
                        mg_module::vars::VarType::of(k.into()).map_or("?", |t| t.name()),
                        d.line,
                        t.name()
                    )),
                    Some(_) => {}
                }
            }
            let listed = areas::list_in(&m, module_codepage(cli, &m));
            for a in &filter.only {
                if !listed.iter().any(|l| l.resref == *a) {
                    bail!("{a} is not an area of the module");
                }
            }
            let chosen: Vec<&areas::AreaInfo> =
                listed.iter().filter(|a| filter.matches(a)).collect();
            if !changes.is_empty() && filter.is_empty() && !*all {
                bail!("name areas or give a filter, or pass --all to change every area");
            }
            let resrefs: Vec<mg_core::ResRef> = chosen.iter().map(|a| a.resref).collect();
            let codepage = module_codepage(cli, &m);
            // What Make Placeables Static leaves dynamic, and why (read
            // before the change, by the project's rules).
            let mut left_dynamic = Vec::new();
            if *static_placeables {
                let rules = m.project_file.make_static.clone();
                for &area in &resrefs {
                    let key = ResKey::new(area, ResType::GIT);
                    let Some(Ok(git)) = m.gff(&key) else { continue };
                    let list = git.root.list("Placeable List").unwrap_or(&[]);
                    for (i, why) in areas::static_plan(&git.root, &rules).kept {
                        let tag = list[i].string("Tag").unwrap_or_default();
                        let tag = codepage.decode(tag).into_owned();
                        left_dynamic.push((area, i, tag, why.describe()));
                    }
                }
            }
            let done =
                areas::apply(&mut m, &resrefs, &changes, codepage).map_err(anyhow::Error::msg)?;
            project_file_notes(&m, &mut out);
            for n in described_notes {
                out.note(n);
            }
            let kinds = |flags: u32| {
                let word = |bit: u32, yes: &'static str, no: &'static str| {
                    if flags & bit != 0 { yes } else { no }
                };
                [
                    word(areas::INTERIOR, "interior", "exterior"),
                    word(areas::UNDERGROUND, "underground", "above ground"),
                    word(areas::NATURAL, "natural", "artificial"),
                ]
                .join(", ")
            };
            if changes.is_empty() {
                for a in &chosen {
                    out.line(format!(
                        "{}\t{}\t{}\t{}",
                        a.resref,
                        a.name,
                        a.tileset,
                        kinds(a.flags)
                    ));
                }
                out.note(format!("{} of {} areas", chosen.len(), listed.len()));
            } else {
                let text = |v: &Option<String>| v.clone().unwrap_or_else(|| "(none)".into());
                for c in &done {
                    out.line(format!(
                        "{}\t{}\t{} -> {}",
                        c.area,
                        c.what,
                        text(&c.from),
                        text(&c.to)
                    ));
                }
                for (area, i, tag, why) in &left_dynamic {
                    out.note(format!("{area}: placeable {i} {tag} left dynamic: {why}"));
                }
                let changed: std::collections::BTreeSet<_> = done.iter().map(|c| c.area).collect();
                if *dry_run {
                    out.note(format!(
                        "would change {} of {} areas chosen",
                        changed.len(),
                        chosen.len()
                    ));
                } else {
                    if !done.is_empty() {
                        m.save()?;
                    }
                    out.note(format!("changed {} of {} areas chosen", changed.len(), chosen.len()));
                }
            }
            out.json = json!({
                "areas": chosen.iter().map(|a| json!({
                    "resref": a.resref.to_string(),
                    "name": a.name,
                    "tag": a.tag,
                    "tileset": a.tileset.to_string(),
                    "interior": a.flags & areas::INTERIOR != 0,
                    "underground": a.flags & areas::UNDERGROUND != 0,
                    "natural": a.flags & areas::NATURAL != 0,
                })).collect::<Vec<_>>(),
                "changes": done.iter().map(|c| json!({
                    "area": c.area.to_string(),
                    "field": c.what,
                    "from": c.from,
                    "to": c.to,
                })).collect::<Vec<_>>(),
                "left_dynamic": left_dynamic.iter().map(|(area, i, tag, why)| json!({
                    "area": area.to_string(),
                    "index": i,
                    "tag": tag,
                    "why": why,
                })).collect::<Vec<_>>(),
                "dry_run": *dry_run,
            });
            out
        }
        Cmd::UpdateInstances { module, blueprints, area } => {
            let mut m = Module::open(module)?;
            let gi = install(cli)?;
            let wanted: Vec<ResKey> = if blueprints.is_empty() {
                m.keys()
                    .copied()
                    .filter(|k| mg_rules::ObjectKind::from_restype(k.restype).is_some())
                    .collect()
            } else {
                blueprints
                    .iter()
                    .map(|b| ResKey::from_filename(b).context("give each blueprint as name.ext"))
                    .collect::<Result<_>>()?
            };
            for k in &wanted {
                if !m.contains(k) {
                    bail!("{k} is not in the module");
                }
            }
            let mut out = Output::default();
            let tlk = mg_tlk::Tlk::read(&std::fs::read(gi.talk_table(false))?)?;
            let game = mg_rules::GameData::new(module_resman(&gi, &m, &mut out)?, tlk);
            let only = area
                .as_deref()
                .map(mg_core::ResRef::from_str)
                .transpose()
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            let mut total = 0;
            let mut by_area = serde_json::Map::new();
            for a in m.areas()? {
                if only.is_some_and(|o| o != a) {
                    continue;
                }
                let key = ResKey::new(a, mg_core::ResType::GIT);
                let Some(Ok(git)) = m.gff(&key) else { continue };
                let read = |k: ResKey| mg_module::gff_root(Some(&m), &game, &k);
                let item = |r: mg_core::ResRef| read(ResKey::new(r, mg_core::ResType::UTI));
                let placing = mg_module::instances::Placing { game: &game, item: &item };
                let blueprint = |t: mg_core::ResType, r: mg_core::ResRef| {
                    let k = ResKey::new(r, t);
                    if wanted.contains(&k) { read(k) } else { None }
                };
                // The items the area's objects hold (in chests, creatures'
                // packs and hands, stores, bags) first, then the objects.
                let held_item = |r: mg_core::ResRef| blueprint(mg_core::ResType::UTI, r);
                let (root, held) =
                    mg_module::instances::update_held(&placing, &git.root, &held_item, &|_| true)
                        .unwrap_or_else(|| (git.root.clone(), 0));
                let updated =
                    mg_module::instances::update(&placing, &root, &blueprint, &|_, _| true)
                        .map_or((root, held), |(new, n)| (new, n + held));
                if let (new, n @ 1..) = updated {
                    let mut g = git.clone();
                    g.root = new;
                    m.set_gff(key, &g)?;
                    out.line(format!("{a}: {n}"));
                    by_area.insert(a.to_string(), n.into());
                    total += n;
                }
            }
            if total > 0 {
                m.save()?;
            }
            out.json = json!({ "updated": total, "areas": by_area, "blueprints": wanted.len() });
            out.note(format!("updated {total} objects from {} blueprints", wanted.len()));
            out
        }
        Cmd::Build { project, target, output, keep_going, ndb } => {
            // A target that packs a talk table: its JSON made into the
            // table, as nasher packs it.
            let package = mg_module::nasher::Package::read(project)?;
            if let Some(t) = target.as_deref().and_then(|name| package.target(name))
                && t.is_talk_table()
            {
                let (path, bytes) = mg_module::nasher::pack_talk_table(project, t)?;
                let path = output.clone().unwrap_or(path);
                if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
                    std::fs::create_dir_all(dir)?;
                }
                std::fs::write(&path, &bytes)?;
                let lines = mg_tlk::Tlk::read(&bytes).map_or(0, |t| t.entries.len());
                let mut out = Output::new(
                    json!({ "compiled": null, "packed": path_text(&path), "lines": lines }),
                );
                out.note(format!("packed {} ({lines} lines)", path.display()));
                return Ok(out);
            }
            let mut m = Module::open_project(project, target.as_deref())?;
            let mut out = Output::default();
            if let Some(p) = &m.project {
                for w in &p.warnings {
                    out.note(format!("warning: {w}"));
                }
            }
            let rm = module_resman(&install(cli)?, &m, &mut out)?;
            let results = mg_module::build::compile_scripts_with(
                &mut m,
                &rm,
                mg_module::build::ScriptSelection::All,
                None,
                *ndb,
            );
            let compile = compiled(&mut out, &results);
            let failed = compile["failed"].as_u64().unwrap_or(0) as usize;
            if failed > 0 && !keep_going {
                // The compiler's messages, then the failure.
                if cli.json {
                    out.json = json!({ "compiled": compile, "packed": null });
                    out.failed = true;
                    return Ok(out);
                }
                for l in &out.text {
                    println!("{l}");
                }
                bail!("{failed} of {} scripts failed to compile", results.len());
            }
            let (path, bytes) = m.target_archive()?.context("not a nasher project")?;
            let path = output.clone().unwrap_or(path);
            if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
                std::fs::create_dir_all(dir)?;
            }
            std::fs::write(&path, bytes)?;
            out.json = json!({ "compiled": compile, "packed": path_text(&path) });
            out.note(format!(
                "compiled {} of {} scripts; packed {}",
                results.len() - failed,
                results.len(),
                path.display()
            ));
            out
        }
        Cmd::Init { module, project } => {
            let mut m = Module::open(module)?;
            m.save_as(&ModuleLocation::Project {
                root: project.clone(),
                target: "default".into(),
            })?;
            let p = m.project.as_ref().context("no project")?;
            let mut out = Output::new(json!({
                "project": path_text(project),
                "resources": m.len(),
                "target": p.target,
                "file": p.target().file,
            }));
            for w in &p.warnings {
                out.note(format!("warning: {w}"));
            }
            out.note(format!(
                "{} resources in {} (target {}, packs {})",
                m.len(),
                project.display(),
                p.target,
                p.target().file
            ));
            out
        }
        Cmd::Import { module, erf, overwrite } => {
            let mut m = Module::open(module)?;
            let data = std::fs::read(erf)?;
            let gi = install(cli)?;
            let mut out = Output::default();
            let rm = module_resman(&gi, &m, &mut out)?;
            let plan = mg_module::transfer::plan_import(&m, &data, &rm)?;
            for r in &plan.missing {
                out.note(format!(
                    "warning: {} needs {:?} {}, found nowhere",
                    r.from, r.kind, r.target
                ));
            }
            // Where the archive came from, and whether the module has moved
            // on since (said before anything is imported).
            let stamp = mg_module::transfer::archive_stamp(&data);
            let base_warning = stamp.as_ref().and_then(|s| mg_module::stamp::compare(s, &m));
            match &stamp {
                Some(s) => out.note(format!("made from {}", s.summary())),
                None => out.note("the archive doesn't say what it was made from (no stamp)"),
            }
            if let Some(w) = &base_warning {
                out.note(format!("warning: {w}"));
            }
            let s = mg_module::transfer::import_erf(&mut m, &data, |_| *overwrite)?;
            m.save()?;
            let names = |v: &[ResKey]| v.iter().map(ToString::to_string).collect::<Vec<_>>();
            out.json = json!({
                "added": names(&s.added),
                "replaced": names(&s.replaced),
                "skipped": names(&s.skipped),
                "new_areas": s.new_areas.iter().map(ToString::to_string).collect::<Vec<_>>(),
                "stamp": stamp.as_ref().map(stamp_json),
                "base_warning": base_warning,
            });
            out.note(format!(
                "imported {} new, replaced {}, skipped {} existing; new areas: {:?}",
                s.added.len(),
                s.replaced.len(),
                s.skipped.len(),
                s.new_areas.iter().map(|a| a.to_string()).collect::<Vec<_>>()
            ));
            out
        }
        Cmd::Attach { module, files, replace } => {
            use mg_module::attach::{There, copy, hak_list, placements};
            let gi = install(cli)?;
            let user = gi.user_dir.context("no user folder; pass --user-dir")?;
            let placed = placements(&user, files).map_err(anyhow::Error::msg)?;
            let differ: Vec<String> = placed
                .iter()
                .filter(|p| p.there == There::Different)
                .map(|p| p.to.display().to_string())
                .collect();
            if !differ.is_empty() && !*replace {
                bail!(
                    "other files of these names are there (--replace replaces them): {}",
                    differ.join(", ")
                );
            }
            let copied = copy(&placed, *replace).map_err(anyhow::Error::msg)?;
            let mut m = Module::open(module)?;
            let mut info = m.info()?;
            let haks: Vec<String> =
                placed.iter().filter(|p| !p.is_tlk).map(|p| p.name.clone()).collect();
            if !haks.is_empty() {
                info.root.set("Mod_HakList", hak_list(&info.root, &haks));
            }
            let tlk = placed.iter().find(|p| p.is_tlk).map(|t| t.name.clone());
            if let Some(t) = &tlk {
                info.root.set("Mod_CustomTlk", mg_gff::Value::String(t.clone().into_bytes()));
            }
            m.set_info(&info)?;
            m.save()?;
            let mut out =
                Output::new(json!({ "copied": copied, "haks": m.haks()?, "custom_tlk": tlk }));
            out.note(format!(
                "copied {copied} file(s); the module has {} hak(s) attached",
                m.haks()?.len()
            ));
            out
        }
        Cmd::Minimap { module, area, out: png, size } => {
            let m = Module::open(module)?;
            let gi = install(cli)?;
            let mut out = Output::default();
            let rm = module_resman(&gi, &m, &mut out)?;
            let key = ResKey::parse(area, ResType::ARE).context("not an area name")?;
            let data = rm.get(&key).with_context(|| format!("{area}.are"))?;
            let are = mg_gff::Gff::read(&data)?;
            let set = mg_module::minimap::tileset(&rm, &are.root).map_err(anyhow::Error::msg)?;
            let image = mg_module::minimap::minimap(&rm, &are.root, &set, *size)
                .map_err(anyhow::Error::msg)?;
            std::fs::write(png, mg_module::minimap::png(&image).map_err(anyhow::Error::msg)?)?;
            out.json =
                json!({ "file": path_text(png), "width": image.width, "height": image.height });
            out.note(format!("{}×{} pixels", image.width, image.height));
            out
        }
        Cmd::Find { module, types, tag, name, resref, area, placed, blueprints, fields } => {
            let m = Module::open(module)?;
            let types = types
                .iter()
                .map(|t| {
                    ResType::from_extension(t.trim_start_matches('.'))
                        .filter(|t| mg_rules::ObjectKind::from_restype(*t).is_some())
                        .with_context(|| format!("{t}: not a blueprint type (utc, utp, …)"))
                })
                .collect::<Result<Vec<_>>>()?;
            // (Names are matched and shown in the module's own letters.)
            let cp = module_codepage(cli, &m);
            let q = mg_module::query::Query {
                types,
                tag: tag.clone(),
                name: (name.as_deref())
                    .map(|n| cp.spelled_in(n, Codepage::WINDOWS_1252).into_owned()),
                resref: resref.clone(),
                area: area
                    .as_deref()
                    .map(mg_core::ResRef::from_str)
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("{e}"))?,
                placed: if *placed {
                    Some(true)
                } else if *blueprints {
                    Some(false)
                } else {
                    None
                },
                fields: fields
                    .iter()
                    .map(|f| match f.split_once('=') {
                        Some((l, v)) => (l.trim().to_string(), Some(v.to_string())),
                        None => (f.trim().to_string(), None),
                    })
                    .collect(),
            };
            let mut found = mg_module::query::find(&m, &q);
            for f in &mut found {
                f.name = Codepage::WINDOWS_1252.spelled_in(&f.name, cp).into_owned();
            }
            let mut out = Output::new(json!({
                "found": found.iter().map(|f| json!({
                    "kind": if f.placed.is_some() { "placed" } else { "blueprint" },
                    "type": f.restype.extension(),
                    "resref": f.resref.map(|r| r.to_lowercase().to_string()),
                    "tag": f.tag,
                    "name": f.name,
                    "area": f.placed.map(|(a, _)| a.to_string()),
                    "index": f.placed.map(|(_, i)| i),
                    "position": f.position,
                })).collect::<Vec<_>>(),
            }));
            for f in &found {
                let what = format!(
                    "{}\t{}\t{}\t{}",
                    f.restype.extension().unwrap_or("?"),
                    f.resref.map(|r| r.to_lowercase().to_string()).unwrap_or_default(),
                    f.tag,
                    f.name
                );
                match (f.placed, f.position) {
                    (Some((a, i)), Some([x, y, z])) => {
                        out.line(format!("{a}[{i}]\t{what}\t{x:.2}, {y:.2}, {z:.2}"))
                    }
                    (Some((a, i)), None) => out.line(format!("{a}[{i}]\t{what}")),
                    _ => out.line(format!("blueprint\t{what}")),
                }
            }
            out.note(format!("{} found", found.len()));
            out
        }
        Cmd::Nwsync {
            module,
            repository,
            with_module,
            uuid,
            name,
            description,
            group_id,
            no_latest,
            limit_file_size,
            force,
            dry_run,
        } => {
            let m = Module::open(module)?;
            let gi = install(cli)?;
            let mut out = Output::default();
            let rm = module_resman(&gi, &m, &mut out)?;
            let info = mg_module::query::info(&m)?;
            let own_uuid = m
                .info()?
                .root
                .string("Mod_UUID")
                .map(|u| String::from_utf8_lossy(u).into_owned())
                .filter(|u| !u.is_empty());
            let uuid = with_module
                .then(|| uuid.clone().or(own_uuid).unwrap_or_else(mg_module::nwsync::new_uuid));
            let (contents, missing) =
                mg_module::nwsync::module_contents(&m, &gi, &rm, *with_module, uuid.as_deref())
                    .map_err(anyhow::Error::msg)?;
            if !missing.is_empty() {
                bail!("haks not found: {}", missing.join(", "));
            }
            let o = mg_module::nwsync::Options {
                with_module: *with_module,
                name: name.clone().unwrap_or(if *with_module { info.name } else { String::new() }),
                description: description.clone().unwrap_or(if *with_module {
                    info.description
                } else {
                    String::new()
                }),
                uuid,
                group_id: *group_id,
                latest: !no_latest,
                limit: (*limit_file_size > 0).then(|| limit_file_size * 1024 * 1024),
                force: *force,
                dry_run: *dry_run,
            };
            let w = mg_module::nwsync::write(repository, &contents, &o, &mut |_, _| {})
                .map_err(anyhow::Error::msg)?;
            out.json = json!({
                "repository": path_text(repository),
                "sha1": w.sha1,
                "files": w.files,
                "bytes": w.bytes,
                "on_disk_bytes": w.on_disk,
                "new_files": w.new_files,
                "dry_run": *dry_run,
            });
            out.line(w.sha1.clone());
            out.note(format!(
                "manifest {}: {} files, {:.1} MB ({:.1} MB compressed, {} data files new){}",
                w.sha1,
                w.files,
                w.bytes as f64 / 1048576.0,
                w.on_disk as f64 / 1048576.0,
                w.new_files,
                if *dry_run { "; nothing written" } else { "" }
            ));
            out.note(format!(
                "serve {} on a web server, and start nwserver with -nwsyncurl <its address>",
                repository.display()
            ));
            out
        }
        Cmd::TilesetPalette { set, output } => {
            let data = std::fs::read(set).with_context(|| path_text(set))?;
            let tileset = mg_set::Tileset::parse(&data, Codepage::default())?;
            let (gff, warnings) = mg_module::tileset::palette(&tileset);
            let stem =
                set.file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
            let path =
                output.clone().unwrap_or_else(|| set.with_file_name(format!("{stem}palstd.itp")));
            std::fs::write(&path, gff.to_bytes()?)?;
            let count = |id: usize| match gff.root.get("MAIN") {
                Some(mg_gff::Value::List(c)) => match c.get(id).and_then(|c| c.get("LIST")) {
                    Some(mg_gff::Value::List(l)) => l.len(),
                    _ => 0,
                },
                _ => 0,
            };
            let mut out = Output::new(json!({
                "palette": path_text(&path),
                "features": count(0),
                "groups": count(1),
                "terrain": count(2),
            }));
            for w in warnings {
                out.note(format!("warning: {w}"));
            }
            out.note(format!(
                "wrote {}: {} features, {} groups, {} terrain",
                path.display(),
                count(0),
                count(1),
                count(2)
            ));
            out
        }
        Cmd::Roundtrip { module, keep } => roundtrip(module, keep.as_deref())?,
        Cmd::ModelFiles { what, out, module, all, force, dry_run } => {
            let gi = install(cli)?;
            model_files(&gi, what, out, module.as_deref(), *all, *force, *dry_run)?
        }
        Cmd::Search(args) => search::run(cli, args)?,
        Cmd::Info { module } => {
            let m = Module::open(module)?;
            let mut i = mg_module::query::info(&m)?;
            let cp = module_codepage(cli, &m);
            for text in [&mut i.name, &mut i.description] {
                *text = Codepage::WINDOWS_1252.spelled_in(text, cp).into_owned();
            }
            let mut out = Output::new(json!({
                "name": i.name,
                "tag": i.tag,
                "description": i.description,
                "entry_area": i.entry_area.map(|a| a.to_string()),
                "areas": i.areas.iter().map(ToString::to_string).collect::<Vec<_>>(),
                "haks": i.haks,
                "custom_tlk": i.custom_tlk,
                "min_game_version": i.min_game_version,
                "resources": i.resources.iter().map(|(t, n)| (t.clone(), json!(n))).collect::<serde_json::Map<_, _>>(),
            }));
            out.line(format!("Name\t{}", i.name));
            out.line(format!("Tag\t{}", i.tag));
            out.line(format!(
                "Entry area\t{}",
                i.entry_area.map(|a| a.to_string()).unwrap_or_default()
            ));
            out.line(format!(
                "Areas\t{}",
                i.areas.iter().map(ToString::to_string).collect::<Vec<_>>().join(", ")
            ));
            out.line(format!("Haks\t{}", i.haks.join(", ")));
            out.line(format!("Talk table\t{}", i.custom_tlk.clone().unwrap_or_default()));
            out.line(format!("Game version\t{}", i.min_game_version));
            let counts: Vec<String> = i.resources.iter().map(|(t, n)| format!("{n} {t}")).collect();
            out.line(format!("Resources\t{}", counts.join(", ")));
            out
        }
        Cmd::Tlk { strrefs } => {
            let gi = install(cli)?;
            let data = std::fs::read(gi.talk_table(false))?;
            let tlk = mg_tlk::Tlk::read(&data)?;
            let texts: Vec<(u32, String)> =
                strrefs.iter().map(|&s| (s, tlk.text(StrRef(s)).unwrap_or_default())).collect();
            let mut out = Output::new(json!({
                "strings": texts.iter().map(|(s, t)| json!({ "strref": s, "text": t })).collect::<Vec<_>>(),
            }));
            for (s, t) in &texts {
                out.line(format!("{s}\t{t}"));
            }
            out
        }
    };
    Ok(out)
}

/// The codepage of a module's text: Windows-1252, or the table of an
/// `encoding.2da` in its haks (which takes the game install to find them:
/// without one, Windows-1252).
fn module_codepage(cli: &Cli, m: &Module) -> Codepage {
    let own = Codepage::WINDOWS_1252;
    let haks = m.haks().unwrap_or_default();
    let Ok(gi) = install(cli) else { return own };
    let Ok(mut rm) = ResMan::for_game(&gi) else { return own };
    let _ = rm.add_haks(&gi, &haks);
    mg_rules::encoding_of(&rm, own)
}

/// The plugins that look at what `mg set` or `mg apply` saves (those
/// `--plugins` names, and the project's as allowed), with the game's data
/// where there is a game. None: no plugin has a handler for a save.
fn save_guard(
    cli: &Cli,
    m: &Module,
    dirs: &[PathBuf],
    out: &mut Output,
) -> Result<Option<plugins::SaveGuard>> {
    // (Only a module of a project has plugins of its own.)
    let project = m.location.as_ref().and_then(mg_module::project_file::ProjectFile::root_of);
    if dirs.is_empty() && project.is_none() {
        return Ok(None);
    }
    let gi = install(cli).ok();
    plugins::save_guard(gi.as_ref(), m, dirs, out)
}

/// An export's stamp ([`mg_module::stamp`]) as JSON.
fn stamp_json(s: &mg_module::stamp::Stamp) -> serde_json::Value {
    serde_json::json!({
        "moonglow": s.version,
        "commit": s.commit.as_ref().map(|c| c.hash.clone()),
        "dirty": s.commit.as_ref().map(|c| c.dirty),
        "module": s.module,
        "tag": s.tag,
    })
}

/// The resman for a module: the game, the module's haks and the module.
fn module_resman(gi: &GameInstall, m: &Module, out: &mut Output) -> Result<ResMan> {
    let mut rm = ResMan::for_game(gi)?;
    // (A module whose hak list can't be read, or a hak that can't be
    // opened, stops the command.)
    m.haks()?;
    let (missing, wrong) = m.stack_onto(&mut rm, gi);
    if let Some(e) = wrong {
        return Err(e.into());
    }
    for missing in missing {
        out.note(format!("warning: hak {missing} not found"));
    }
    Ok(rm)
}

/// A group of `mg verify --skip`, by its id.
fn verify_group(id: &str) -> std::result::Result<mg_module::doctor::Group, String> {
    use mg_module::doctor::Group;
    Group::from_id(id).ok_or_else(|| {
        let ids: Vec<&str> = Group::ALL.iter().map(|g| g.id()).collect();
        format!("not a group: {id} (one of {})", ids.join(", "))
    })
}

/// `mg verify --fix`: the checks whose findings' fixes to make, and
/// whether only to say so.
struct Fixes<'a> {
    checks: &'a [String],
    dry_run: bool,
}

/// Makes the fixes of the findings of the checks asked for (each on the
/// module as it was found, a command each, held to what Verify read:
/// [`mg_edit::fix::Guard`]) and saves. The findings fixed, by place in
/// `found`, and the report's lines and JSON.
fn make_fixes(
    m: &Module,
    found: &[mg_edit::fix::Found],
    asked: &Fixes<'_>,
    out: &mut Output,
) -> Result<(Vec<usize>, serde_json::Value, Vec<String>)> {
    use mg_edit::fix::Guard;
    let chosen: Vec<(usize, &mg_edit::fix::Fix)> = found
        .iter()
        .enumerate()
        .filter(|(_, f)| asked.checks.iter().any(|c| *c == f.finding.check))
        .filter_map(|(i, f)| Some((i, f.fix.as_ref()?)))
        .collect();
    for check in asked.checks {
        if !chosen.iter().any(|(i, _)| found[*i].finding.check == *check) {
            out.note(format!("warning: --fix {check}: no finding of that check has a fix"));
        }
    }
    let mut guard = Guard::new(m, chosen.iter().map(|(_, f)| *f));
    let mut ws = mg_edit::Workspace::new(m.clone());
    let (mut fixed, mut made, mut refused) = (Vec::new(), Vec::new(), Vec::new());
    let mut lines = Vec::new();
    for (i, fix) in chosen {
        let f = &found[i].finding;
        let about = serde_json::json!({
            "check": f.check, "resource": f.resource.to_string(), "at": f.at,
            "label": fix.label,
        });
        let why = match guard.check(&mut ws, fix) {
            Ok(()) => ws.apply(fix.command()).map_err(|e| e.to_string()),
            Err(why) => Err(why),
        };
        match why {
            Ok(()) => {
                guard.made(&mut ws, fix);
                let verb = if asked.dry_run { "would fix" } else { "fixed" };
                lines.push(format!("{verb}\t{} › {}: {}", f.check, f.resource, fix.label));
                fixed.push(i);
                made.push(about);
            }
            Err(why) => {
                out.note(format!("warning: not fixed: {} › {}: {why}", f.check, f.resource));
                let mut about = about;
                about["why"] = serde_json::json!(why);
                refused.push(about);
            }
        }
    }
    if !fixed.is_empty() && !asked.dry_run {
        ws.save()?;
    }
    let made = serde_json::json!({ "made": made, "refused": refused, "dry_run": asked.dry_run });
    Ok((fixed, made, lines))
}

/// A fix as `mg verify --json` says it: what its button says, and its
/// edits as `mg apply` takes them.
fn fix_json(fix: &mg_edit::fix::Fix, codepage: Codepage) -> serde_json::Value {
    match mg_edit::wire::command_to_json(&fix.command(), codepage) {
        Ok(command) => serde_json::json!({ "label": fix.label, "command": command }),
        Err(e) => serde_json::json!({ "label": fix.label, "error": e.to_string() }),
    }
}

fn verify(
    gi: &GameInstall,
    path: &Path,
    show_unused: bool,
    plugins: &plugins::Chosen<'_>,
    skip: &[mg_module::doctor::Group],
    asked: &Fixes<'_>,
    times: &mut mg_module::doctor::Timings,
) -> Result<Output> {
    use mg_module::doctor::Group;
    let m = times.time("open the module", || Module::open(path))?;
    let mut out = Output::default();
    project_file_notes(&m, &mut out);
    let rm = times.time("the game and the haks", || module_resman(gi, &m, &mut out))?;
    // (Every GFF of the module read once for both.)
    let references = if show_unused || !skip.contains(&Group::References) {
        times.time("the module's references", || mg_module::verify::module_references(&m))
    } else {
        Vec::new()
    };
    let unused =
        if show_unused { mg_module::verify::unused_of(&m, &references) } else { Vec::new() };
    let mut missing = if skip.contains(&Group::References) {
        Vec::new()
    } else {
        times.time("missing references", || mg_module::verify::missing_of(&m, &rm, references))
    };
    let count = |data: Vec<u8>| mg_tlk::Tlk::read(&data).map(|t| t.entries.len()).ok();
    let (base, custom) = times.time("the talk tables", || {
        let base = std::fs::read(gi.talk_table(false)).ok().and_then(count).unwrap_or(0);
        let custom = m.custom_tlk().ok().flatten().and_then(|name| {
            let language = mg_core::Language::from_short_code(&gi.language);
            mg_module::talk::find_any(&m, &rm, &gi.tlk_dirs(), &name, language)
                .and_then(|f| f.tlk_bytes().ok())
                .and_then(count)
        });
        (base, custom)
    });
    let tlk = mg_module::doctor::TalkTables { base, custom };
    let mut findings = mg_module::doctor::examine_groups(&m, &rm, tlk, skip, times);
    // The plugins' checks, beside the doctor's (none run when they are
    // left out).
    let theirs = if skip.contains(&Group::Plugins) {
        Vec::new()
    } else {
        times.time("plugins", || plugins::findings(gi, &m, plugins, &mut out))?
    };
    findings.extend(theirs.iter().map(|f| f.finding.clone()));
    let left = mg_module::doctor::leave_out(&mut missing, &mut findings, skip);
    // Each finding with its fix, if it has one.
    let found = mg_edit::fix::attach(&m, findings, &theirs);
    let codepage = mg_rules::encoding_of(&rm, Codepage::WINDOWS_1252);
    let (fixed, made, fix_lines) = if asked.checks.is_empty() {
        (Vec::new(), serde_json::Value::Null, Vec::new())
    } else {
        make_fixes(&m, &found, asked, &mut out)?
    };
    let findings: Vec<&mg_module::doctor::Finding> = found.iter().map(|f| &f.finding).collect();
    let errors = missing.iter().filter(|x| x.is_error()).count()
        + findings.iter().filter(|f| f.severity == mg_module::doctor::Severity::Error).count();
    let warnings = findings.len() + missing.len() - errors;
    // (An error fixed is not one left: the exit says what is.)
    let errors_left = errors
        - fixed
            .iter()
            .filter(|i| {
                !asked.dry_run && found[**i].finding.severity == mg_module::doctor::Severity::Error
            })
            .count();
    let severity = |s: mg_module::doctor::Severity| match s {
        mg_module::doctor::Severity::Error => "error",
        mg_module::doctor::Severity::Warning => "warning",
    };
    out.json = serde_json::json!({
        "module": path.display().to_string(),
        "errors": errors,
        "warnings": warnings,
        "missing": missing.iter().map(|x| serde_json::json!({
            "severity": if x.is_error() { "error" } else { "warning" },
            "category": format!("{:?}", x.category),
            "from": x.reference.from.to_string(),
            "path": x.reference.path,
            "kind": x.reference.kind.name(),
            "target": x.reference.target.to_string(),
            "uncompiled": x.uncompiled,
            "only_userpatch": x.only_userpatch,
        })).collect::<Vec<_>>(),
        "findings": found.iter().map(|x| { let f = &x.finding; serde_json::json!({
            "severity": severity(f.severity),
            "check": f.check,
            "source": f.source,
            "resource": f.resource.to_string(),
            "at": f.at,
            "message": f.message,
            "fix": x.fix.as_ref().map(|fix| fix_json(fix, codepage)),
        })}).collect::<Vec<_>>(),
        "unused": unused.iter().map(|k| k.to_string()).collect::<Vec<_>>(),
        // (A group left out isn't checked: null, nothing is counted of it.)
        "skipped": left.iter().map(|g| (g.id().to_string(), serde_json::Value::Null)).collect::<serde_json::Map<_, _>>(),
    });
    if !made.is_null()
        && let serde_json::Value::Object(o) = &mut out.json
    {
        o.insert("fixes".into(), made);
    }
    for x in &missing {
        let what = x.what();
        let sev = if x.is_error() { "error" } else { "warning" };
        out.line(format!(
            "{sev}\t{:?}\t{}{}\t{} {} {what}",
            x.category,
            x.reference.from,
            x.reference.path,
            x.reference.kind.name(),
            x.reference.target
        ));
    }
    out.line(format!("{} missing references", missing.len()));
    for x in &found {
        let f = &x.finding;
        let at = if f.at.is_empty() { String::new() } else { format!(" {}", f.at) };
        let fix = match &x.fix {
            Some(fix) => format!(" [fix: {}; --fix {}]", fix.label, f.check),
            None => String::new(),
        };
        out.line(format!(
            "{}\t{} › {}{at}: {}{fix}",
            severity(f.severity),
            f.source,
            f.resource,
            f.message
        ));
    }
    out.line(format!("{} content problems", findings.len()));
    fix_lines.into_iter().for_each(|l| out.line(l));
    for g in &left {
        out.line(format!("left out\t{}: not checked", g.id()));
    }
    if show_unused {
        for k in &unused {
            out.line(format!("unused\t{k}"));
        }
        out.line(format!("{} unused resources", unused.len()));
    }
    if errors_left > 0 {
        out.failed = true;
        out.note(format!("mg: {} has errors", path.display()));
    }
    Ok(out)
}

/// `mg model-files`: the files the game draws a model or a blueprint with.
fn model_files(
    gi: &GameInstall,
    what: &str,
    dir: &Path,
    module: Option<&Path>,
    all: bool,
    force: bool,
    dry_run: bool,
) -> Result<Output> {
    let mut out = Output::default();
    let m = match module {
        Some(path) => Module::open(path)?,
        None => Module::new(),
    };
    let game = plugins::game_for(Some(gi), &m, &mut out).context("the game data")?;
    let preview = match ResKey::from_filename(what) {
        Some(key) if key.restype == ResType::MDL => {
            mg_preview::Preview::model(&key.resref.to_string())
        }
        Some(key) => {
            let data = game.resman.get(&key).with_context(|| format!("{key} is found nowhere"))?;
            let object = Gff::read(&data)?.root;
            let item = |r: mg_core::ResRef| {
                let data = game.resman.get(&ResKey::new(r, ResType::UTI)).ok()?;
                Gff::read(&data).ok()
            };
            let preview = match key.restype {
                ResType::UTP => mg_preview::placeable(&game, &object),
                ResType::UTD => mg_preview::door(&game, &object),
                ResType::UTI => mg_preview::item(&game, &object),
                ResType::UTC => mg_preview::creature(&game, &object, &item),
                _ => bail!("{key}: give a model's name or a .utp, .utd, .uti or .utc"),
            };
            let preview = preview.map_err(|e| anyhow::anyhow!("{key}: {e}"))?;
            mg_preview::replaced(preview, &object)
        }
        None => mg_preview::Preview::model(what),
    };
    let files = mg_preview::files::files(&game, &preview);
    if files.is_empty() {
        bail!("{what}: no model found");
    }
    let stock = files.iter().filter(|f| f.stock).count();
    let every = all || stock == files.len();
    let chosen: Vec<_> = files.iter().filter(|f| every || !f.stock).collect();
    let existing: Vec<String> =
        chosen.iter().map(|f| f.key.to_string()).filter(|name| dir.join(name).exists()).collect();
    if !existing.is_empty() && !force && !dry_run {
        bail!(
            "{} has {} of these files already ({}): --force writes over them",
            dir.display(),
            existing.len(),
            existing.join(", ")
        );
    }
    if !dry_run {
        std::fs::create_dir_all(dir)?;
    }
    for f in &chosen {
        if !dry_run {
            std::fs::write(f.key.path_in(dir)?, game.resman.get(&f.key)?)?;
        }
        out.line(format!("{}\t{}", f.key, f.origin));
    }
    let left_out = files.len() - chosen.len();
    out.json = serde_json::json!({
        "folder": path_text(dir),
        "written": !dry_run,
        "files": chosen
            .iter()
            .map(|f| serde_json::json!({
                "file": f.key.to_string(), "from": f.origin, "stock": f.stock,
            }))
            .collect::<Vec<_>>(),
        "left_out": left_out,
    });
    let done = if dry_run { "would be written" } else { "written" };
    out.note(match left_out {
        0 => format!("{} files {done}", chosen.len()),
        n => format!(
            "{} files {done}; {n} of the game's own left out (--all for them)",
            chosen.len()
        ),
    });
    Ok(out)
}

/// `mg roundtrip`: a module saved and read back, and each GFF and 2DA of
/// it written anew, compared with what it was.
fn roundtrip(path: &Path, keep: Option<&Path>) -> Result<Output> {
    let original = Module::open(path)?;
    let copy = match keep {
        Some(k) => k.to_path_buf(),
        None => std::env::temp_dir().join(format!("mg-roundtrip-{}.mod", std::process::id())),
    };
    if keep.is_some() && copy.exists() {
        bail!("{} is there already", copy.display());
    }
    let mut differences: Vec<(String, String)> = Vec::new();
    let mut differs = |key: &ResKey, what: &str| differences.push((key.to_string(), what.into()));

    // Saved as it is: every resource byte for byte.
    let saved = original.clone().save_as(&ModuleLocation::Archive(copy.clone()));
    let back = saved.map_err(anyhow::Error::from).and_then(|()| Ok(Module::open(&copy)?));
    if keep.is_none() {
        let _ = std::fs::remove_file(&copy);
    }
    let back = back.context("the copy could not be saved and read back")?;
    for k in original.keys() {
        match back.get(k) {
            None => differs(k, "missing from the saved copy"),
            Some(b) if Some(b) != original.get(k) => differs(k, "other bytes in the saved copy"),
            Some(_) => {}
        }
    }
    for k in back.keys().filter(|k| !original.contains(k)) {
        differs(k, "in the saved copy alone");
    }

    // Written anew, as an edit writes it: the same when read back.
    let cp = Codepage::WINDOWS_1252;
    let (mut gffs, mut tables, mut unread) = (0, 0, Vec::new());
    for k in original.keys() {
        let data = original.get(k).expect("its own key");
        if k.restype.is_gff() {
            let read = match Gff::read(data) {
                Ok(g) => g,
                Err(e) => {
                    unread.push((k.to_string(), e.to_string()));
                    continue;
                }
            };
            gffs += 1;
            match read.to_bytes() {
                Err(e) => differs(k, &format!("cannot be written: {e}")),
                Ok(written) => match Gff::read(&written) {
                    Err(e) => differs(k, &format!("written, it cannot be read: {e}")),
                    Ok(again) if again != read => differs(k, "other fields once written"),
                    Ok(again) if again.to_bytes().ok().as_ref() != Some(&written) => {
                        differs(k, "other bytes each time it is written")
                    }
                    Ok(_) => {}
                },
            }
        } else if k.restype == ResType::TWODA {
            let read = match mg_2da::TwoDa::parse(data, cp) {
                Ok(t) => t,
                Err(e) => {
                    unread.push((k.to_string(), e.to_string()));
                    continue;
                }
            };
            tables += 1;
            match read.to_bytes(cp) {
                Err(e) => differs(k, &format!("cannot be written: {e}")),
                Ok(written) => match mg_2da::TwoDa::parse(&written, cp) {
                    Err(e) => differs(k, &format!("written, it cannot be read: {e}")),
                    Ok(again) if again != read => differs(k, "other cells once written"),
                    Ok(_) => {}
                },
            }
        }
    }

    let mut out = Output::new(serde_json::json!({
        "module": path_text(path),
        "resources": original.len(),
        "gffs": gffs,
        "tables": tables,
        "differences": differences
            .iter()
            .map(|(k, what)| serde_json::json!({ "resource": k, "what": what }))
            .collect::<Vec<_>>(),
        "unread": unread
            .iter()
            .map(|(k, why)| serde_json::json!({ "resource": k, "why": why }))
            .collect::<Vec<_>>(),
        "copy": keep.map(path_text),
    }));
    for (k, what) in &differences {
        out.line(format!("{k}\t{what}"));
    }
    for (k, why) in &unread {
        out.note(format!("warning: {k} cannot be read ({why}): kept as it is, never written anew"));
    }
    out.note(format!(
        "{} resources saved and read back, {gffs} GFFs and {tables} 2DAs written anew: {}",
        original.len(),
        match differences.len() {
            0 => "all the same".to_string(),
            n => format!("{n} differ"),
        }
    ));
    out.failed = !differences.is_empty();
    Ok(out)
}

fn gff(input: &Path, output: Option<&Path>, json: bool) -> Result<Output> {
    let data = std::fs::read(input).with_context(|| input.display().to_string())?;
    let cp = Codepage::WINDOWS_1252;
    if input.extension().is_some_and(|e| e.eq_ignore_ascii_case("json")) {
        let value: serde_json::Value = serde_json::from_slice(&data)?;
        let gff = mg_gff::from_json(&value, cp)?;
        let out = output.context("writing GFF needs --output")?;
        std::fs::write(out, gff.to_bytes()?)?;
        return Ok(Output::new(serde_json::json!({ "written": path_text(out) })));
    }
    let is_gff = input
        .extension()
        .and_then(|e| e.to_str())
        .and_then(ResType::from_extension)
        .is_none_or(ResType::is_gff);
    if !is_gff {
        bail!("{} is not a GFF file type", input.display());
    }
    let gff = Gff::read(&data)?;
    let value = mg_gff::to_json(&gff, cp)?;
    match output {
        Some(o) => {
            std::fs::write(o, serde_json::to_string_pretty(&value)? + "\n")?;
            Ok(Output::new(serde_json::json!({ "written": path_text(o) })))
        }
        // The GFF as JSON is the output, with --json or without.
        None if json => Ok(Output::new(value)),
        None => {
            let mut out = Output::default();
            out.line(serde_json::to_string_pretty(&value)?);
            Ok(out)
        }
    }
}
