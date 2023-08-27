// Dependencies
use terminal_menu::{run, menu, label, scroll, string, submenu, back_button, button, mut_menu, list};
use zip_extensions::zip_create_from_directory;
use std::{path::PathBuf, fs::{self, File}, io::{Cursor, Write}};
use platform_dirs::AppDirs;
use crx_dl::{ChromeCRXQuery, crx_to_zip};
use regex::{Regex, Captures};
use lazy_static::lazy_static;

// Constants
const PROXIES_URL: &str = "https://raw.githubusercontent.com/skyyiscool/RoGoldPatcher/main/proxies.txt";
lazy_static! {
    static ref RE: Regex = Regex::new(r"(?m)(setTimeout\(async\(\)=>\{let \w=await(?:.+)Session Invalidated(?:.+)location\.reload\(\)\}\)\}\},1e3\))").unwrap();
    static ref RE2: Regex = Regex::new(r#"(?m)https://www\.rogold\.live/api/info/"\+(\w)"#).unwrap();
    static ref RE3: Regex = Regex::new(r#"(?m)(Object\.values\(await (?:\w+)\("(?:\w+)",)\(\)=>(?:[\w=]+)\("(?:[\w=]+)",null,!0\)"#).unwrap();
}

/// Fetches each proxy.
fn get_proxies() -> Vec<String> {
    reqwest::blocking::Client::new()
        .get(PROXIES_URL)
        .send()
        .expect("unable to grab proxies")
        .text()
        .expect("invalid proxies")
        .lines()
        .map(|x| x.to_string())
        .collect()
}

/// Performs the entire patching process.
fn patch(path: PathBuf, proxy: String) {
    // Adding the proxy to the manifest
    let manifest = path.join("manifest.json");
    let manifest_contents = fs::read_to_string(&manifest).expect("Unable to open file (manifest.json)")
        .replace("*://*.rogold.live/*", &format!("*://*.rogold.live/*\",\n\t\t\"{}*", proxy.replace("https", "*")));
    fs::write(&manifest, manifest_contents).expect("Unable to write file contents (manifest.json)");

    // Patching the main file
    let main_file = path.join("src/main.js");
    let mut main_file_contents = fs::read_to_string(&main_file).expect("Unable to open file (src/main.js)")
        .replace("https://www.rogold.live/api/info/css", &format!("{}css.css", proxy));

    main_file_contents = RE2.replace(&main_file_contents, |caps: &Captures| {
        format!("{}\"+{}+\".css\"", &proxy, &caps[1])
    }).to_string();

    main_file_contents = RE3.replace(&main_file_contents, |caps: &Captures| {
        format!("{}async()=>{{return [\"aaa\",\"aaa\",0,0]}}", &caps[1])
    }).to_string();

    // Final patch and write
    fs::write(
        &main_file,
        RE.replace_all(&main_file_contents, "setTimeout(async()=>{},1e3)").to_string()
    ).expect("Unable to write file contents (src/main.js)");
}

/// Downloads RoGold source.
fn download_extension() -> Vec<u8> {
    // Download the extension
    let mut crx_query = ChromeCRXQuery::default();
    crx_query.x = "namkakclifhmccmkbeddddpjegmdimed";
    let extension_crx = crx_query.download_blocking().unwrap();

    // Convert it to .zip
    let crx_zip = crx_to_zip(extension_crx, None).unwrap();

    // Done
    crx_zip
}

/// Downloads RoGold source, then output to file as `.zip`.
fn download_extract() {
    // Download the extension's source
    let extension_source = download_extension();

    // Output to file
    let mut file_out = File::create(format!("{}.zip", "RoGold")).unwrap();
    file_out.write_all(&extension_source).unwrap();

    // Output
    println!("Downloaded RoGold.");
}

/// Downloads RoGold source and patches automatically.
fn download_patch(selected_proxy: String) {
    // Download the extension's source
    let extension_source = download_extension();

    // Extract the extension
    let extract_dir = PathBuf::from("RoGold");
    zip_extract::extract(Cursor::new(extension_source), &extract_dir, true).unwrap();

    // Patch
    patch(extract_dir, selected_proxy.to_string());
    println!("Finished patching.");
}

/// Entrypoint.
fn main() {
    // Grab all proxies
    let proxies = get_proxies();

    // Grab vars, checking if using automated process
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 2 {
        // Figure out which proxy we are using
        let arg = args.get(1).unwrap();
        let selected_proxy = if arg.chars().next().unwrap().is_numeric() {
            proxies
                .get(arg.parse::<usize>().unwrap())
                .expect("unable to get proxy index")
        } else {
            arg
        };

        // Download and patch
        download_patch(selected_proxy.to_string());

        // Zip up
        let source_dir = PathBuf::from("RoGold");
        zip_create_from_directory(&PathBuf::from("RoGold-PATCHED.zip"), &source_dir)
            .expect("unable to create zip");

        // Delete directory
        fs::remove_dir_all(source_dir)
            .expect("unable to remove RoGold folder");

        // Done
        return
    }

    // Construct the menu and run it
    let menu = menu(vec![
        label("-------------------------"),
        label("-     RoGold Patcher     -"),
        label("- Created by Stefanuk12 -"),
        label("-------------------------"),
        submenu("Custom Patch", vec![
            label      ("-----------------------------------"),
            label      ("-     RoGold Patcher - Patcher     -"),
            label      ("-      Created by Stefanuk12      -"),
            label      ("-----------------------------------"),
            scroll     ("Select a proxy", proxies.clone()),
            string     ("Custom proxy (overwrites)", "", true),
            label      ("--------------"),
            string     ("RoGold Path", "./", false),
            list       ("Use Opera GX Path", vec!["No", "Yes"]),
            label      ("--------------"),
            button     ("Start"),
            back_button("Back")
        ]),
        button("Download RoGold source as .zip"),
        button("Download and Patch (uses default proxy)"),
        back_button("Exit")
    ]);
    run(&menu);

    // User has exited, process their action
    let mut mm = mut_menu(&menu);
    let selected_item = mm.selected_item_name();
    match selected_item {
        "Exit" => return println!("Goodbye!"),
        "Download RoGold source as .zip" => download_extract(),
        "Download and Patch (uses default proxy)" => download_patch(proxies.get(0).unwrap().to_string()),
        "Custom Patch" => {
            // Grab their selected proxy
            let patch_menu = mm.get_submenu("Custom Patch");
            let custom_proxy = patch_menu.selection_value("Custom proxy (overwrites)");
            let selected_proxy = if custom_proxy.is_empty() { patch_menu.selection_value("Select a proxy") } else { custom_proxy }; 
            
            // Grab their selected path
            let selected_path = if patch_menu.selection_value("Use Opera GX Path") == "Yes" {
                let ext_path = AppDirs::new(Some(r"Opera Software\Opera GX Stable\Extensions\namkakclifhmccmkbeddddpjegmdimed"), false).unwrap().config_dir;
                fs::read_dir(ext_path)
                    .expect("extension not installed?")
                    .flatten()
                    .filter(|x| x.metadata().unwrap().is_dir())
                    .max_by_key(|x| x.metadata().unwrap().modified().unwrap())
                    .unwrap()
                    .path()
            } else {
                PathBuf::from(patch_menu.selection_value("RoGold Path"))
            };

            // Patch
            patch(selected_path, selected_proxy.to_string());
            println!("Finished patching.");
        }
        _ => return println!("You should not be seeing this...")
    };
}
