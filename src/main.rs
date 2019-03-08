#![feature(vec_remove_item)]

use std::cmp;
use std::collections::btree_map::BTreeMap;
use std::env;
use std::fs;
use std::thread;

type TagId = usize;

type PhotoId = usize;

#[derive(Debug, Clone, PartialEq)]
struct HorizontalPhoto {
    pub id: PhotoId,
    pub tags: Vec<TagId>,
}

#[derive(Debug, Clone, PartialEq)]
struct VerticalPhoto {
    pub id: PhotoId,
    pub tags: Vec<TagId>,
}

type Score = usize;

#[derive(Debug, Clone, PartialEq)]
enum Slide<'a> {
    Single(&'a HorizontalPhoto),
    Dual(&'a VerticalPhoto, &'a VerticalPhoto),
}

impl<'a> Slide<'a> {
    pub fn len(&self) -> usize {
        match self {
            Slide::Single(s) => s.tags.len(),
            Slide::Dual(s1, s2) => {
                let mut len = s1.tags.len() + s2.tags.len();
                for t in s1.tags.iter() {
                    if s2.tags.contains(t) {
                        len -= 1;
                    }
                }
                len
            }
        }
    }

    pub fn contains(&self, t: &TagId) -> bool {
        match self {
            Slide::Single(s) => s.tags.contains(t),
            Slide::Dual(s1, s2) => s1.tags.contains(t) || s2.tags.contains(t),
        }
    }
}

#[derive(Debug)]
struct Slideshow<'a> {
    slides: Vec<Slide<'a>>,
}

struct TagMap {
    i: usize,
    inner: BTreeMap<String, TagId>,
}

impl TagMap {
    pub fn new() -> TagMap {
        TagMap {
            i: 0,
            inner: BTreeMap::new(),
        }
    }

    pub fn add(&mut self, tag: String) -> TagId {
        match self.inner.get(&tag) {
            Some(t) => t.clone(),
            None => {
                let result: TagId = self.i;
                self.inner.insert(tag, result.clone());
                self.i += 1;
                result
            }
        }
    }
}

impl<'a> Slideshow<'a> {
    fn score(&self) -> Score {
        let mut score = 0;
        for i in 1..self.slides.len() {
            score += subscore(&self.slides[i - 1], &self.slides[i]);
        }
        score
    }

    fn write(&self, path: String) {
        let mut result: String = format!("{}\n", self.slides.len());
        for slide in self.slides.iter() {
            match &slide {
                Slide::Single(hphoto) => {
                    result += &format!("{}\n", hphoto.id);
                }
                Slide::Dual(vphoto1, vphoto2) => {
                    result += &format!("{} {}\n", vphoto1.id, vphoto2.id);
                }
            }
        }

        let output_path = path.replace("txt", "result");
        fs::write(output_path, result).expect("Could not write output file.");
    }
}

fn subscore(x: &Slide, y: &Slide) -> Score {
    let mut same = 0;
    match x {
        Slide::Single(xh) => {
            for tx in xh.tags.iter() {
                if y.contains(tx) {
                    same += 1;
                }
            }
        }
        Slide::Dual(xv1, xv2) => {
            match y {
                Slide::Single(yh) => {
                    for ty in yh.tags.iter() {
                        if x.contains(ty) {
                            same += 1;
                        }
                    }
                }
                Slide::Dual(_, _) => {
                    for tx in xv1.tags.iter() {
                        if y.contains(tx) {
                            same += 1;
                        }
                    }
                    for tx in xv2.tags.iter() {
                        if y.contains(tx) && !xv1.tags.contains(tx) {
                            same += 1;
                        }
                    }
                }
            };
        }
    };

    same.min((x.len() - same).min(y.len() - same))
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut paths: Vec<String> = vec![
        "data/a_example.txt".to_string(),
        "data/b_lovely_landscapes.txt".to_string(),
        "data/c_memorable_moments.txt".to_string(),
        "data/d_pet_pictures.txt".to_string(),
        "data/e_shiny_selfies.txt".to_string(),
    ];

    if args.len() > 1 {
        paths = vec![args[1].clone()];
    }

    let mut jobs = vec![];

    for path in paths.into_iter() {
        let job = thread::spawn(move || {
            let contents = fs::read_to_string(&path).expect("Could not read file.");
            let lines: Vec<&str> = contents.lines().collect();

            let mut tag_map = TagMap::new();
            let mut hphotos: Vec<HorizontalPhoto> = vec![];
            let mut vphotos: Vec<VerticalPhoto> = vec![];

            lines[1..].iter().enumerate().for_each(|(id, buf)| {
                let pieces: Vec<&str> = buf.split(' ').collect();
                let mut tags: Vec<TagId> = pieces[2..]
                    .iter()
                    .map(|t| tag_map.add(t.to_string()))
                    .collect();
                tags.sort();

                if pieces[0] == "V" {
                    vphotos.push(VerticalPhoto { id, tags });
                } else {
                    hphotos.push(HorizontalPhoto { id, tags });
                }
            });

            let hphotos = hphotos;
            let vphotos = vphotos;

            let mut hindexes: Vec<usize> = (0..hphotos.len()).collect();
            let mut vindexes: Vec<usize> = (0..vphotos.len()).collect();
            let mut resulting: Vec<Slide> = vec![];

            if hphotos.len() > 0 {
                resulting.push(Slide::Single(&hphotos[hindexes.remove(0)]))
            } else {
                resulting.push(Slide::Dual(&vphotos[vindexes.remove(0)], &vphotos[vindexes.remove(0)]))
            }

            loop {
                let last = resulting.last().unwrap();

                let mut best_h_score = 0;
                let mut best_h_index = None;
                for i in 0..cmp::min(40000, hindexes.len()) {
                    let slide = Slide::Single(&hphotos[hindexes[i]]);
                    let score = subscore(last, &slide);
                    if best_h_index.is_none() || score > best_h_score {
                        best_h_score = score;
                        best_h_index = Some(i);
                    }
                }

                let mut best_v_score = 0;
                let mut best_v_index = None;
                for i in 0..cmp::min(1000, vindexes.len()) {
                    for j in 0..cmp::min(10, vindexes.len()) {
                        if i != j {
                            let slide = Slide::Dual(&vphotos[vindexes[i]], &vphotos[vindexes[j]]);
                            let score = subscore(last, &slide);
                            if best_v_index.is_none() || score > best_v_score {
                                best_v_score = score;
                                best_v_index = Some((i, j));
                            }
                        }
                    }
                }

                if hindexes.len() % 2000 == 1000 || vindexes.len() % 2000 == 1000 {
                    println!(
                        "{} horizontal and {} vertical photos todo",
                        hindexes.len(),
                        vindexes.len()
                    );
                }

                if best_h_score >= best_v_score && best_h_index.is_some() {
                    match best_h_index {
                        None => break,
                        Some(i) => resulting.push(Slide::Single(&hphotos[hindexes.remove(i)])),
                    }
                } else {
                    match best_v_index {
                        None => break,
                        Some((i, j)) => {
                            resulting.push(
                                if i < j {
                                    Slide::Dual(&vphotos[vindexes.remove(i)], &vphotos[vindexes.remove(j - 1)])
                                } else {
                                    Slide::Dual(&vphotos[vindexes.remove(i)], &vphotos[vindexes.remove(j)])
                                }
                            );
                        }
                    }
                }
            }

            let slide_show = Slideshow { slides: resulting };
            println!("Found score {} for {}", slide_show.score(), &path);
            slide_show.write(path.to_string());
        });

        jobs.push(job);
    }

    for j in jobs {
        let _ = j.join();
    }
}
