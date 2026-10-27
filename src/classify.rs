use crate::config::Config;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    Note,
    Journal,
    Todo,
}

#[derive(Debug, PartialEq)]
pub struct Classified<'c, 't> {
    pub kind: Kind,
    pub domain: &'c str,
    pub text: &'t str,
}

/// Leading type and domain keywords (either order, both optional) are matched and removed.
pub fn classify<'c, 't>(cfg: &'c Config, text: &'t str) -> Classified<'c, 't> {
    let ws = words(text);
    let types = [
        (Kind::Note, &cfg.types.note),
        (Kind::Journal, &cfg.types.journal),
        (Kind::Todo, &cfg.types.todo),
    ];
    let types = types
        .iter()
        .flat_map(|(k, kws)| kws.iter().map(move |kw| (*k, kw.as_str())));
    let domains = cfg
        .domains
        .iter()
        .flat_map(|(d, v)| v.keywords.iter().map(move |kw| (d.as_str(), kw.as_str())));

    let (mut kind, mut domain, mut pos) = (None, None, 0);
    for _ in 0..2 {
        if kind.is_none()
            && let Some((k, n)) = best(&ws[pos..], types.clone())
        {
            (kind, pos) = (Some(k), pos + n);
            continue;
        }
        if domain.is_none()
            && let Some((d, n)) = best(&ws[pos..], domains.clone())
        {
            (domain, pos) = (Some(d), pos + n);
            continue;
        }
        break;
    }

    let text = match pos {
        0 => text.trim(),
        _ => text[ws[pos - 1].1..]
            .trim_start_matches(|c: char| !c.is_alphanumeric())
            .trim_end(),
    };
    Classified {
        kind: kind.unwrap_or(Kind::Note),
        domain: domain.unwrap_or(&cfg.default_domain),
        text,
    }
}

/// Longest keyword matching the first words, with its word count.
fn best<'k, T>(
    ws: &[(String, usize)],
    cands: impl Iterator<Item = (T, &'k str)>,
) -> Option<(T, usize)> {
    cands
        .filter_map(|(v, kw)| {
            let kw: Vec<String> = words(kw).into_iter().map(|w| w.0).collect();
            let hit = !kw.is_empty()
                && ws.len() >= kw.len()
                && ws.iter().zip(&kw).all(|(w, k)| w.0 == *k);
            hit.then_some((v, kw.len()))
        })
        .max_by_key(|(_, n)| *n)
}

/// Folded words with the byte offset where each ends in `s`.
fn words(s: &str) -> Vec<(String, usize)> {
    let mut out = Vec::new();
    let mut start = None;
    for (i, c) in s.char_indices().chain([(s.len(), ' ')]) {
        if c.is_alphanumeric() {
            start.get_or_insert(i);
        } else if let Some(st) = start.take() {
            out.push((fold(&s[st..i]), i));
        }
    }
    out
}

// ponytail: French accents only, unicode-normalization crate if other languages need it.
fn fold(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            c => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> Config {
        Config::parse(include_str!("../config.example.yaml")).unwrap()
    }

    fn check(input: &str, kind: Kind, domain: &str, text: &str) {
        let cfg = cfg();
        assert_eq!(
            classify(&cfg, input),
            Classified { kind, domain, text },
            "input: {input:?}"
        );
    }

    #[test]
    fn keywords() {
        check(
            "Journal ESA, réunion avec Paul.",
            Kind::Journal,
            "esa",
            "réunion avec Paul.",
        );
        check(
            "À faire maison, changer le filtre",
            Kind::Todo,
            "maison",
            "changer le filtre",
        );
        check("a faire: appeler", Kind::Todo, "perso", "appeler");
        check("Todo travail rapport", Kind::Todo, "esa", "rapport");
        check(
            "Tâche. NVC lire le livre",
            Kind::Todo,
            "cnv",
            "lire le livre",
        );
        check(
            "CNV journal. Bonne écoute",
            Kind::Journal,
            "cnv",
            "Bonne écoute",
        );
        check("Note perso idée", Kind::Note, "perso", "idée");
        check("To do, ESA: budget", Kind::Todo, "esa", "budget");
        check("Afaire maison vitres", Kind::Todo, "maison", "vitres");
        check("Affaire maison vitres", Kind::Todo, "maison", "vitres");
        check("A faire ESA budget", Kind::Todo, "esa", "budget");
    }

    #[test]
    fn defaults_and_no_false_match() {
        check("  Penser au pain  ", Kind::Note, "perso", "Penser au pain");
        check("Notes de cours", Kind::Note, "perso", "Notes de cours");
        check(
            "Maisonnette à vendre",
            Kind::Note,
            "perso",
            "Maisonnette à vendre",
        );
        check("Journal", Kind::Journal, "perso", "");
        check("", Kind::Note, "perso", "");
    }
}
