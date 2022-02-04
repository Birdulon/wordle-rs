use std::fs;
use std::collections::HashMap;
use bitintr::{Lzcnt, Tzcnt};
use regex::Regex;
use rayon::prelude::*;

const WORD_LENGTH: usize = 5;
type Charmask = i32;

#[derive(Copy, Clone, Default)]
struct SimState {
    banned_chars: [Charmask; WORD_LENGTH],  // Alphabetical bitmask
    required_chars: Charmask
}

#[derive(Copy, Clone, Default)]
struct Word {
    letters: [char; WORD_LENGTH],
    charmask: Charmask  // All of the characters contained
}

type WordCache = HashMap<Charmask, Vec<Word>>;

fn str2word(s: &str) -> Word {
    let mut word = Word::default();
    let mut iter = s.chars();
    for i in 0..WORD_LENGTH {
        let c = iter.next().unwrap();
        word.letters[i] = c;
        word.charmask |= char2bit(c);
    }
    word
}

fn load_dictionary(filename: &str) -> Vec<Word> {
    println!("Loading dictionary at {}", filename);
    let rawfile = fs::read_to_string(filename).unwrap();
    let rawwords = rawfile.split('\n');
    let mut words = Vec::new();
    let re = Regex::new(&format!("{}{}{}", r"^[A-Za-z]{", WORD_LENGTH, r"}$")).unwrap();
    for line in rawwords {
        if re.is_match(line) {
            words.push(line.to_uppercase());
        }
    }
    words.sort();
    words.dedup();
    words.iter().map(|w| str2word(w)).collect()
}

/* fn inc_char(c: char) -> char {
    (c as u8 + 1) as char
} */

/* fn hs2str(hs: &HashSet<char>) -> String {
    let mut chars: Vec<char> = hs.iter().cloned().collect();
    if chars.is_empty() {
        "".to_string()
    } else {
        chars.sort_unstable();
        chars.iter().collect()
    }
} */


fn char2bit(c: char) -> Charmask {
    debug_assert!(('A'..='Z').contains(&c));
    1 << (c as u8 - 'A' as u8)
}

fn cm2char(cm: Charmask, offset: i8) -> char {
    (((31 - cm.lzcnt() as i8) + 'A' as i8 + offset) as u8) as char
}

fn _generate_wordcache_nested(cache: &mut WordCache, subcache: &[Word], key: Charmask, depth: u8) {
    for c in cm2char(key, 1)..='Z' {
        let cb = char2bit(c);
        let sc2: Vec<Word> = subcache.iter().filter(|w| (w.charmask & cb) == cb).cloned().collect();
        if !sc2.is_empty() {
            let key2 = key | cb;
            if depth > 0 {
                _generate_wordcache_nested(cache, &sc2, key2, depth-1);
            }
            cache.insert(key2, sc2);
        }
    }
}

fn generate_wordcache(words: Vec<Word>) -> WordCache {
    let mut cache: WordCache = HashMap::new();
    let subcache: Vec<Word> = words.iter().cloned().collect();
    _generate_wordcache_nested(&mut cache, &subcache, 0, 5);
    cache.insert(0, words);
    cache
}

fn filter_word(w: &Word, banned_chars: &[Charmask; 5], required_chars: Charmask) -> bool {
    if w.charmask & required_chars != required_chars {
        return false;
    }
    for (c, bans) in w.letters.iter().zip(banned_chars.iter()) {
        if char2bit(*c) & bans != 0 {
            return false;
        }
    }
    true
}

fn simulate(guess: &Word, solution: &Word, mut s: SimState, wordcache: &WordCache) -> (Vec<Word>, SimState) {
    s.required_chars |= guess.charmask & solution.charmask;
    for (i, (gc, sc)) in guess.letters.iter().zip(solution.letters.iter()).enumerate() {
        let gb = char2bit(*gc);
        if gc == sc {  // Right letter right position
            s.banned_chars[i] = 255 ^ gb;
        } else if solution.charmask & gb != 0 {  // Right letter wrong position
            s.banned_chars[i] |= gb;
        } else {  // Letter not in solution
            for j in 0..s.banned_chars.len() {
                s.banned_chars[j] |= gb;
            }
        }
    }
    let cachekey = s.required_chars;
    match wordcache.contains_key(&cachekey) {
        true => (
            wordcache[&cachekey].iter().filter(|w| filter_word(w, &s.banned_chars, s.required_chars)).cloned().collect(),
            s
        ),
        false => (
            Vec::<Word>::new(),
            s
        ),
    }
}

fn find_worstcase(word: &Word, wordcache: &WordCache) -> (String, usize) {
    let mut worst = 0;
    let ss = SimState::default();
    for target in &wordcache[&0] {
        let remaining = simulate(word, target, ss, &wordcache).0.len();
        if remaining > worst {worst = remaining};
    }
    let wordstr: String = word.letters.iter().collect();
    let output = format!("{} - {}", wordstr, worst);
    println!("{}", output);
    (output, worst)
}

fn main() {
    fs::write("test.txt", ["test1", "test2", "test3"].join("\n")).expect("Failed to write output");
    let words = load_dictionary("words");
    println!("Hello, world! {} words in dict", words.len());
    let wordcache = generate_wordcache(words);
    //let sr = simulate(&wordcache[""][0], &wordcache[""][5000], &wordcache);
    //println!("{:?}", sr);
    let mut results: Vec<(String, usize)> = wordcache[&0].par_iter().map(|w| find_worstcase(w, &wordcache)).collect();
    results.sort_by_key(|r| r.1);
    let results_strs: Vec<String> = results.iter().map(|r| r.0.clone()).collect();
    fs::write("results.txt", results_strs.join("\n")).expect("Failed to write output");
    //println!("{:?}", wordcache.keys());
}
