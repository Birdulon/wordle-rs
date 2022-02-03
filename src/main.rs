use std::fs;
use std::collections::HashMap;
use regex::Regex;

fn load_dictionary(filename: &str) -> Vec<String> {
    println!("Loading dictionary at {}", filename);
    let rawfile = fs::read_to_string(filename).unwrap();
    let rawwords = rawfile.split('\n');
    let mut words = Vec::new();
    let re = Regex::new(r"^\w{5}$").unwrap();
    for line in rawwords {
        if re.is_match(line) {
            words.push(line.to_uppercase());
        }
    }
    words
}

fn inc_char(c: char) -> char {
    (c as u8 + 1) as char
}

fn _generate_wordcache_nested(cache: &mut HashMap<String, Vec<String>>, subcache: &[String], key: &str, depth: u8) {
    for c in inc_char(key.chars().last().unwrap())..='Z' {
        let sc2: Vec<String> = subcache.iter().filter(|w| w.contains(c)).cloned().collect();
        if !sc2.is_empty() {
            let key2 = format!("{}{}", key, c);
            if depth > 0 {
                _generate_wordcache_nested(cache, &sc2, &key2, depth-1);
            }
            cache.insert(key2, sc2);
        }
    }
}

fn generate_wordcache(words: Vec<String>) -> HashMap<String, Vec<String>> {
    let mut cache = HashMap::new();
    for c1 in 'A'..='Z' {
        let sc: Vec<String> = words.iter().filter(|w| w.contains(c1)).cloned().collect();
        if !sc.is_empty() {
            let key = format!("{}", c1);
            _generate_wordcache_nested(&mut cache, &sc, &key, 4);
            cache.insert(key, sc);
        }
    }
    cache.insert("".to_string(), words);
    cache
}

fn main() {
    let words = load_dictionary("/usr/share/dict/words");
    println!("Hello, world! {} words in dict", words.len());
    let wordcache = generate_wordcache(words);
    println!("{:?}", wordcache.keys());
}
