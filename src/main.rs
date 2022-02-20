#![allow(dead_code)]
#![allow(unused_imports)]
use std::{env, fs};
// use bitintr::Lzcnt;
use regex::Regex;
use rayon::prelude::*;
use itertools::zip;
use array_init::array_init;

pub type Charmask = i32;
pub type Achar = i8;  // ASCII char

pub const WORD_LENGTH: usize = 5;
pub const WORD_LENGTH_P: usize = 5;  // Padded for SIMD shenanigans
pub const GUESS_DEPTH: usize = 1;  // TODO: Change this whenever working at different depths
pub const N_SOLUTIONS: usize = 2315;
pub const CACHE_SIZE: usize = 1<<26;
pub const IDX_ALL_WORDS: Charmask = (CACHE_SIZE as Charmask) - 1;
pub const IDX_VALID_SOLUTIONS: Charmask = 0;
pub const A: Achar = 'A' as Achar;
pub const Z: Achar = 'Z' as Achar;

pub const MAX_ENTRIES_PER_JOB: usize = 1000;

#[derive(Copy, Clone, Default)]
pub struct Word {
    charbits: [Charmask; WORD_LENGTH_P],  // Each letter in bitmask form
    charmask: Charmask,                   // All of the characters contained
    //letters: [Achar; WORD_LENGTH]
}


#[cfg(use_thin_array)]
mod thin_array;
#[cfg(use_thin_array)]
type WordCache = thin_array::ThinArray<Vec<Word>, CACHE_SIZE, 7000>;

#[cfg(all(not(use_thin_array), use_hashmap))]
use std::collections::HashMap;
#[cfg(all(not(use_thin_array), use_hashmap))]
type WordCache = HashMap<Charmask, Vec<Word>>;  // Default hash is slower than BTree on M1

#[cfg(all(not(use_thin_array), not(use_hashmap), feature = "ahash"))]
use std::collections::HashMap;
#[cfg(all(not(use_thin_array), not(use_hashmap), feature = "ahash"))]
use ahash::{AHasher, RandomState};
#[cfg(all(not(use_thin_array), not(use_hashmap), feature = "ahash"))]
type WordCache = HashMap<Charmask, Vec<Word>, RandomState>;

#[cfg(all(not(use_thin_array), not(use_hashmap), not(feature = "ahash"), feature = "xxhash_rust"))]
use std::collections::HashMap;
#[cfg(all(not(use_thin_array), not(use_hashmap), not(feature = "ahash"), feature = "xxhash_rust"))]
use xxhash_rust::xxh3::Xxh3;
#[cfg(all(not(use_thin_array), not(use_hashmap), not(feature = "ahash"), feature = "xxhash_rust"))]
use std::hash::BuildHasherDefault;
#[cfg(all(not(use_thin_array), not(use_hashmap), not(feature = "ahash"), feature = "xxhash_rust"))]
type WordCache = HashMap<Charmask, Vec<Word>, BuildHasherDefault<Xxh3>>;

#[cfg(all(not(use_thin_array), not(use_hashmap), not(feature = "ahash"), not(feature = "xxhash_rust")))]
use std::collections::BTreeMap;
#[cfg(all(not(use_thin_array), not(use_hashmap), not(feature = "ahash"), not(feature = "xxhash_rust")))]
type WordCache = BTreeMap<Charmask, Vec<Word>>;


fn default_wordcache() -> WordCache {
    WordCache::default()
}


fn char2bit(c: Achar) -> Charmask {
    debug_assert!((A..=Z).contains(&c));
    1 << (c - A)
}

fn str2word(s: &str) -> Word {
    let mut word = Word::default();
    let mut iter = s.chars();
    for i in 0..WORD_LENGTH {
        let c = iter.next().unwrap() as Achar;
        let cb = char2bit(c);
        word.charbits[i] = cb;
        //word.letters[i] = c;
        word.charmask |= cb;
    }
    word
}

/* fn cm2char(cm: Charmask, offset: i8) -> Achar {
    (((31 - cm.lzcnt() as i8) + A + offset) as u8) as Achar
}

fn letters2str(letters: [Achar; WORD_LENGTH]) -> String {
    letters.iter().map(|x| (*x as u8) as char).collect()
}

fn charbits2str(charbits: [Charmask; WORD_LENGTH]) -> String {
    charbits.iter().map(|x| (cm2char(*x, 0) as u8) as char).collect()
} */

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

/* fn charmask2str(cm: Charmask) -> String {
    let mut s = String::default();
    for i in cm.tzcnt() ..= 32-cm.lzcnt() {
        if (cm & (1<<i)) != 0 {
            s += &((A + i as Achar) as u8 as char).to_string();
        }
    }
    s
} */

fn load_dictionary(filename: &str) -> Vec<String> {
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
    //words.sort();
    //words.dedup();
    words
}

fn _generate_wordcache_nested(cache: &mut WordCache, subcache: &[Word], key: Charmask, next_c: Achar, depth: u8) {
    for c in next_c..=Z {
        let cb = char2bit(c);
        let sc2: Vec<Word> = subcache.iter().filter(|w| (w.charmask & cb) == cb).cloned().collect();
        if !sc2.is_empty() {
            let key2 = key | cb;
            if depth > 0 {
                _generate_wordcache_nested(cache, &sc2, key2, c+1, depth-1);
            }
            cache.insert(key2, sc2);
        }
    }
}

fn generate_wordcache(valid_words: Vec<Word>) -> WordCache {
    let mut cache: WordCache = default_wordcache();
    let valid_solutions: Vec<Word> = valid_words[..N_SOLUTIONS].to_vec();  // Hacky way to separate the valid solutions from the larger guessing list
    _generate_wordcache_nested(&mut cache, &valid_solutions, 0, A, 5);
    cache.insert(IDX_VALID_SOLUTIONS, valid_solutions);
    cache.insert(IDX_ALL_WORDS, valid_words);
    cache
}

fn filter_word(w: &[Charmask; WORD_LENGTH_P], banned_chars: &[Charmask; WORD_LENGTH_P]) -> bool {
    zip(w, banned_chars).all(|(x,y)| x & y == 0)
}

fn aggregate_guesses(guess_ids: &Vec<usize>, wordcache: &WordCache) -> Word {
    //guess_ids.iter().reduce(|out, g| out |= wordcache[IDX_ALL_WORDS][g]).unwrap()
    let all_words = &wordcache[&IDX_ALL_WORDS];
    let mut iter = guess_ids.iter();
    let mut aggregate_guess = all_words[*iter.next().unwrap()];
    for g in iter {
        let guess = all_words[*g];
        for i in 0..aggregate_guess.charbits.len() {
            aggregate_guess.charbits[i] |= guess.charbits[i];
        }
        aggregate_guess.charmask |= guess.charmask;
    }
    aggregate_guess
}

fn simulate(guess: Word, wordcache: &WordCache) -> (usize, usize, Vec<usize>) {
    // let valid_words = &wordcache[&IDX_ALL_WORDS];
    let valid_solutions = &wordcache[&IDX_VALID_SOLUTIONS];

    let required_chars: [Charmask; N_SOLUTIONS] = array_init::from_iter(
        valid_solutions.iter().map(|s| s.charmask & guess.charmask)
    ).unwrap();
    let mut banned_chars: [Charmask; WORD_LENGTH*N_SOLUTIONS] = [0; WORD_LENGTH*N_SOLUTIONS];
    /* array_init::from_iter(
        valid_solutions.iter().map(|s| s.charmask & guess.charmask)
    ).unwrap(); */
    for i in 0..N_SOLUTIONS {
        let s = valid_solutions[i];
        let bans = guess.charmask & !s.charmask;  // A letter fully rejected in any position bans it in all positions
        for j in 0..WORD_LENGTH {
            banned_chars[i*WORD_LENGTH + j] = bans;
            banned_chars[i*WORD_LENGTH + j] |= guess.charbits[j] & !s.charbits[j];  // A letter in the wrong position
            // A correct letter bans all others in the position. TODO: test branchless toggle
            let correct = guess.charbits[j] & s.charbits[j];
            //Branch
            /* if correct != 0 {
                banned_chars[i*WORD_LENGTH + j] |= !correct;    
            } */
            //Branchless
            banned_chars[i*WORD_LENGTH + j] |= !correct * (correct !=0) as i32;
        }
    }

    let mut worst = 0;
    let mut worst_w = 0;
    let mut worst_list_w: Vec<&Word> = vec![];
    for target_id in 0..N_SOLUTIONS {   
        let cachekey = required_chars[target_id];
        if wordcache.contains_key(&cachekey) {
            let mut remaining = 0;
            let mut words = vec![];
            for word in &wordcache[&cachekey] {
                // TODO: test branchless toggle
                let mut error = 0;
                for c in 0..WORD_LENGTH {
                    error += word.charbits[c] & banned_chars[target_id*WORD_LENGTH + c];
                }
                // remaining += (error == 0) as usize;
                if error == 0 {
                    remaining += 1;
                    words.push(word);
                }
            }
            if remaining > worst {
                worst = remaining;
                worst_w = target_id;
                worst_list_w = words;
            }
        }
    }
    let worst_list = worst_list_w.iter().map(|w| valid_solutions.iter().position(|x| x.charbits==w.charbits).unwrap()).collect();
    (worst, worst_w, worst_list)
}

// fn calculate_best(w1start: usize, w1end: usize, total: usize, wordcache: &WordCache) -> Vec<(Vec<usize>, (usize, usize))> {
//     println!("Starting from word #{} to ending word #{}.", w1start, w1end);
//     let mut guess_ids: Vec<Vec<usize>> = Vec::default();
//     for i1 in w1start..w1end {
//         for i2 in i1..total {
//             guess_ids.push(vec![i1,i2])
//         }
//     }
//     let guesses: Vec<Word> = guess_ids.iter().map(|i| aggregate_guesses(&i, &wordcache)).collect();
//     println!("This consists of {} guess combinations", guess_ids.len());

//     let mut results: Vec<(Vec<usize>, (usize, usize))> =
//         (0..guess_ids.len()).into_par_iter()
//         .map(|i| (guess_ids[i].clone(), simulate(guesses[i], &wordcache)))
//         .collect();
//     // results.sort_by_key(|(_guess, (worst, _solution))| worst);
//     results.sort_by_key(|x| x.1.0);
//     println!("Processed {} guesses from starting word #{} to ending word #{}.", results.len(), w1start, w1end);
//     results
// }

fn agg_guesses(w1: &Word, w2: &Word) -> Word {
    let mut g: Word = *w1;
    for i in 0..g.charbits.len() {
        g.charbits[i] |= w2.charbits[i];
    }
    g.charmask |= w2.charmask;
    g
}

fn aggregate_guesses2(guess_ids: &[usize], wordcache: &WordCache) -> Vec<Word> {
    //guess_ids.iter().reduce(|out, g| out |= wordcache[IDX_ALL_WORDS][g]).unwrap()
    let all_words = &wordcache[&IDX_ALL_WORDS];
    let mut iter = guess_ids.iter();
    let mut aggregate_guess = all_words[*iter.next().unwrap()];
    for g in iter {
        let guess = all_words[*g];
        for i in 0..aggregate_guess.charbits.len() {
            aggregate_guess.charbits[i] |= guess.charbits[i];
        }
        aggregate_guess.charmask |= guess.charmask;
    }

    all_words.iter().map(|w| agg_guesses(&aggregate_guess, w)).collect()
}

fn calculate_best2(seed: &[usize], wordcache: &WordCache) -> Vec<(usize, (usize, usize, Vec<usize>))> {
    println!("Aggregating guesses");
    let guesses: Vec<Word> = aggregate_guesses2(seed, &wordcache);
    println!("This consists of {} guess combinations", guesses.len());

    let mut results: Vec<(usize, (usize, usize, Vec<usize>))> =
        (0..guesses.len()).into_par_iter()
        .map(|i| (i, simulate(guesses[i], &wordcache)))
        .collect();
    // results.sort_by_key(|(_guess, (worst, _solution))| worst);
    results.sort_by_key(|x| x.1.0);
    println!("Processed {} guesses.", results.len());
    results
}

fn guess2str(guess: &[usize], word_strs: &[String]) -> String {
    let strs: Vec<String> = guess.iter().map(|i| word_strs[*i].clone()).collect();
    strs.join(",")
}

fn find_word_id_from_str(s: &str, words: &Vec<Word>) -> usize {
    let w = str2word(s);
    words.iter().position(|x| x.charbits==w.charbits).unwrap()
}

fn format_ids(ids: &Vec<usize>, word_strs: &Vec<String>) -> String {
    let s: Vec<String> = ids.iter().map(|i| word_strs[*i].to_string()).collect();
    s.join(",")
}

fn main() {
    eprint!("Hello, world!\n");
    // Prints each argument on a separate line
    for argument in env::args() {
        print!("{}\t", argument);
    }
    fs::write("test.txt", ["test1", "test2", "test3"].join("\n")).expect("Failed to write output");
    let word_strs: Vec<String> = load_dictionary("words-kura");
    let totalwords = word_strs.len();
    let words: Vec<Word> = word_strs.iter().map(|w| str2word(w)).collect();
    println!("Loaded dict - {} words in dict", totalwords);
    let wordcache = generate_wordcache(words);
    //let all_words = &wordcache[&IDX_ALL_WORDS];
    // println!("Cache contains {} keys", wordcache.keys().len());  // 6756 on words-kura

    // let args: Vec<String> = env::args().collect();
    // let mut w1start: usize = 0;
    // let mut w1end: usize = totalwords.min(1000);
    // match args.len() {
    //     3 => {
    //         let s_w1start = &args[1];
    //         let s_w1end = &args[2];
    //         // parse the numbers
    //         w1start = match s_w1start.parse() {
    //             Ok(n) => n,
    //             Err(_) => {
    //                 eprintln!("error: not a valid start point");
    //                 return;
    //             },
    //         };
    //         w1end = match s_w1end.parse() {
    //             Ok(n) => totalwords.min(n),
    //             Err(_) => {
    //                 eprintln!("error: not a valid end point");
    //                 return;
    //             },
    //         };
    //     },
    //     _ => {
    //         w1start = 0;
    //     }
    // }
    
    // Depth-1 full
    //let mut results: Vec<(String, usize)> = (0..totalwords).into_par_iter().map(|i| simulate(all_words[i], &wordcache)).collect();
    // for _ in 0..9 {  // Benching
    //     results = (0..totalwords).into_par_iter().map(|i| simulate(all_words[i], &wordcache)).collect();
    // }

    // Depth-3 (word1,word2,?)
    println!("Finding seed words...");
    let i1: usize = 10186-3;  //find_word_id_from_str("SALET", &wordcache[&0]);
    let i2: usize = 4191-3;  //find_word_id_from_str("COURD", &wordcache[&0]);
    let i3: usize = 285-1;  //find_word_id_from_str("NYMPH", &wordcache[&0]);
    let i4: usize = 924-1;  //find_word_id_from_str("BILGE", &wordcache[&0]);
    println!("Seed words found");

    let results: Vec<(usize, (usize, usize, Vec<usize>))> = calculate_best2(&[i1, i2, i3], &wordcache);
    println!("\tBest score {}, worst {}.", results[0].1.0, results.last().unwrap().1.0);
    let results_strs: Vec<String> = results.iter().map(
        |(guess, (worst, solution, ids))|
        format!("{}\t{} ({}) ({})", worst, guess2str(&[i1,i2,i3,*guess], &word_strs), word_strs[*solution], format_ids(ids, &word_strs))
    ).collect();
    fs::write(format!("results_multi_{}_{}_{}.txt", i1, i2, i3), results_strs.join("\n")).expect("Failed to write output");

    // let results: Vec<(usize, (usize, usize, Vec<usize>))> = calculate_best2(&[i1, i2, i3, i4], &wordcache);
    // println!("\tBest score {}, worst {}.", results[0].1.0, results.last().unwrap().1.0);
    // let results_strs: Vec<String> = results.iter().map(|(guess, (worst, solution, ids))|  format!("{}\t{} ({})", worst, guess2str(&[i1,i2,i3,i4,*guess], &word_strs), word_strs[*solution])).collect();
    // fs::write(format!("results_multi_{}_{}_{}_{}.txt", i1, i2, i3, i4), results_strs.join("\n")).expect("Failed to write output");

    // let results = calculate_best(w1start, w1end, totalwords, &wordcache);
    // let trim = (results.len() - MAX_ENTRIES_PER_JOB).max(0);
    // println!("\tBest score {}, worst {}. Discarding worst {} entries to leave maximum of {}.", results[0].1.0, results.last().unwrap().1.0, trim, MAX_ENTRIES_PER_JOB);
    // let results_strs: Vec<String> = 
    //     results.iter().take(MAX_ENTRIES_PER_JOB.min(results.len()))
    //     .map(
    //         |(guess, (worst, solution))|  format!("{}\t{} ({})", worst, guess2str(guess, &word_strs), word_strs[*solution])
    //     ).collect();
    

    //let results_strs: Vec<String> = results.iter().map(|r| r.0.clone()).collect();
    // fs::write(format!("results_from_{}_to_{}.txt", w1start, w1end), results_strs.join("\n")).expect("Failed to write output");
}
