//! A small CLI over the core: the dictation's models, a WAV file written
//! down, post-processing tried on words, and the newest release. It exists
//! so the core can be checked without a window and driven from a shell.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("help");
    match cmd {
        "clean" => {
            // kotoba-core clean <key> <words | file of lines>: the dictation's
            // clean-up, as the app runs it, on words given here or on a
            // file a dictation a line: what the model wrote, and whether
            // it was taken (`clean::accept`). The instructions are the
            // ones in the settings.
            use kotoba_core::clean;
            let app = kotoba_core::config::Config::load().app;
            let m = clean::model(args.get(1).map(String::as_str).unwrap_or(clean::DEFAULT_MODEL)).expect("no such model");
            let given = args.get(2).expect("clean <key> <words | file>");
            let lines: Vec<String> = match std::fs::read_to_string(given) {
                Ok(text) => text.lines().filter(|l| !l.trim().is_empty()).map(str::to_string).collect(),
                Err(_) => vec![given.clone()],
            };
            #[cfg(target_os = "macos")]
            {
                for words in lines {
                    let at = std::time::Instant::now();
                    let words = kotoba_core::dictate::avoid(&kotoba_core::dictate::spell(&words, &app.dictate_terms), &app.dictate_avoid);
                    match clean::rewrite(m, &words, &app.clean_prompt, &app.dictate_terms) {
                        Ok(wrote) => {
                            let wrote = kotoba_core::dictate::avoid(&kotoba_core::dictate::spell(&wrote, &app.dictate_terms), &app.dictate_avoid);
                            let taken = clean::accept(&words, &wrote).is_some();
                            println!("SAID   {words}\nWROTE  {}\n       {} in {:.2} s\n", wrote.trim().replace('\n', "\n       "), if taken { "taken" } else { "REFUSED" }, at.elapsed().as_secs_f32());
                        }
                        Err(e) => println!("SAID   {words}\nFAILED {e}\n"),
                    }
                }
                clean::unload();
            }
            #[cfg(not(target_os = "macos"))]
            {
                let _ = (m, lines, app);
                println!("dictation is on a Mac only for now");
            }
        }
        "dictate" => {
            // kotoba-core dictate models | download [key] | file <wav> [key]:
            // the dictation's models and post-processing's, one downloaded
            // as the settings window does it (a post-processing model by
            // its key too), and the words in a WAV file (16-bit), as the
            // app in front gets them. The microphone is the app's alone.
            use kotoba_core::{clean, dictate};
            let key = |at: usize| args.get(at).map(String::as_str).unwrap_or(dictate::DEFAULT_MODEL).to_string();
            match args.get(1).map(String::as_str) {
                Some("download") => {
                    let m = dictate::model(&key(2)).or_else(|| clean::model(&key(2))).expect("no such model");
                    let got = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
                    let stop = std::sync::atomic::AtomicBool::new(false);
                    let seen = std::sync::Arc::clone(&got);
                    let ticker = std::thread::spawn(move || loop {
                        let n = seen.load(std::sync::atomic::Ordering::Relaxed);
                        eprint!("\r{} of {} MB", n / 1_000_000, m.bytes / 1_000_000);
                        if n >= m.bytes {
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(500));
                    });
                    let done = dictate::download(m, &got, &stop);
                    got.store(m.bytes, std::sync::atomic::Ordering::Relaxed);
                    let _ = ticker.join();
                    eprintln!();
                    println!("{}", done.map(|_| format!("{} is at {}", m.name, dictate::path_of(m).display())).unwrap_or_else(|e| format!("not downloaded: {e}")));
                }
                Some("file") => {
                    let path = args.get(2).expect("dictate file <wav> [key]");
                    let m = dictate::model(&key(3)).expect("no such model");
                    let bytes = std::fs::read(path).expect("could not read the file");
                    let (samples, channels, rate) = dictate::wav_samples(&bytes).expect("not a 16-bit WAV file");
                    let audio = dictate::to_whisper(&samples, channels, rate);
                    #[cfg(target_os = "macos")]
                    {
                        let at = std::time::Instant::now();
                        let app = kotoba_core::config::Config::load().app;
                        let terms = app.dictate_terms;
                        match dictate::transcribe(m, &audio, &terms).map(|w| dictate::avoid(&dictate::spell(&w, &terms), &app.dictate_avoid)) {
                            Ok(words) => println!("{words}\n({:.1} s of sound in {:.1} s)", audio.len() as f32 / dictate::RATE as f32, at.elapsed().as_secs_f32()),
                            Err(e) => println!("no words: {e}"),
                        }
                        dictate::unload();
                    }
                    #[cfg(not(target_os = "macos"))]
                    {
                        let _ = (m, audio);
                        println!("dictation is on a Mac only for now");
                    }
                }
                _ => {
                    for m in dictate::MODELS.iter().chain(clean::MODELS) {
                        println!("{}  {}  {} MB  {}", m.key, m.name, m.bytes / 1_000_000, if dictate::installed(m) { "downloaded" } else { "not downloaded" });
                    }
                }
            }
        }
        "update" => {
            // kotoba-core update: what the newest release is, against this
            // build. A probe of the check the app makes once a day.
            let current = kotoba_core::update::current_version();
            match kotoba_core::update::latest() {
                Ok(latest) if kotoba_core::update::is_newer(&latest, current) => println!("{latest} is available (this is {current}): {}", kotoba_core::update::release_page(&latest)),
                Ok(latest) => println!("{current} is the newest release (GitHub has {latest})"),
                Err(e) => println!("could not check: {e}"),
            }
        }
        _ => {
            eprintln!("usage: kotoba-core dictate [models | download [key] | file <wav> [key]] | clean <key> <words | file> | update");
        }
    }
}
