use std::fs;
use std::collections::{HashMap, HashSet};
use regex::Regex;
use rayon::prelude::*;

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
    words.sort();
    words.dedup();
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

fn hs2str(hs: &HashSet<char>) -> String {
    let mut chars: Vec<char> = hs.iter().cloned().collect();
    if chars.is_empty() {
        "".to_string()
    } else {
        chars.sort_unstable();
        chars.iter().collect()
    }
}

fn simulate(guess: &str, solution: &str, wordcache: &HashMap<String, Vec<String>>) -> Vec<String> {
    //let b_guess = guess.as_bytes();
    //let b_solution = solution.as_bytes();
    let mut matching_chars = ['.', '.', '.', '.', '.'];
    let mut banned_chars = [HashSet::new(), HashSet::new(), HashSet::new(), HashSet::new(), HashSet::new()];
    let mut required_chars = HashSet::new();
    for (i, (g, s)) in guess.chars().zip(solution.chars()).enumerate() {
        if g == s {  // Right letter right position
            matching_chars[i] = g;
            required_chars.insert(g);
        } else if solution.contains(g) {  // Right letter wrong position
                banned_chars[i].insert(g);
                required_chars.insert(g);
        } else {  // Letter not in solution
            for j in 0..banned_chars.len() {
                banned_chars[j].insert(g);
            }
        }
    }
    let mut re_str = String::new();
    for (m, b) in matching_chars.iter().zip(banned_chars.iter()) {
        if *m != '.' {
            re_str.push(*m);
        } else {
            re_str += &format!("[^{}]", hs2str(b));
        }
    }
    let re = Regex::new(&re_str).unwrap();
    let cachekey = hs2str(&required_chars);
    match wordcache.contains_key(&cachekey) {
        true => wordcache[&cachekey].iter().filter(|w| re.is_match(w)).cloned().collect(),
        false => Vec::<String>::new(),
    }
}

fn find_worstcase(word: &str, wordcache: &HashMap<String, Vec<String>>) -> String {
    let mut worst = 0;
    for target in &wordcache[""] {
        let remaining = simulate(&word, target, &wordcache).len();
        if remaining > worst {worst = remaining};
    }
    let output = format!("{} - {}", word, worst);
    println!("{}", output);
    output
}

fn main() {
    fs::write("test.txt", ["test1", "test2", "test3"].join("\n")).expect("Failed to write output");
    let words = load_dictionary("words");
    println!("Hello, world! {} words in dict", words.len());
    let wordcache = generate_wordcache(words);
    //let sr = simulate(&wordcache[""][0], &wordcache[""][5000], &wordcache);
    //println!("{:?}", sr);
    let results: Vec<String> = wordcache[""].par_iter().map(|w| find_worstcase(w, &wordcache)).collect();
    fs::write("results.txt", results.join("\n")).expect("Failed to write output");
    //println!("{:?}", wordcache.keys());
}
