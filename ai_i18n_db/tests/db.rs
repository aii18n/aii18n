use std::{fs, path::PathBuf, rc::Rc};

use ai_i18n_db::{Db, Error};
use compio::runtime::spawn;

fn db_path(dir: &tempfile::TempDir) -> PathBuf {
  dir.path().join("i18n.aii18n")
}

fn lines(v: &[&str]) -> Vec<String> {
  v.iter().map(|s| (*s).to_owned()).collect()
}

#[compio::test]
async fn set_get_roundtrip() {
  let dir = tempfile::tempdir().unwrap();
  let db = Db::open(db_path(&dir)).unwrap();

  db.set("src/a.md", "zh", &["你好", "世界"]).await.unwrap();
  db.set("src/a.md", "en", &["hello"]).await.unwrap();
  db.set("src/b.md", "zh", &["第二个"]).await.unwrap();

  assert_eq!(
    db.get("src/a.md", "zh").await.unwrap(),
    lines(&["你好", "世界"])
  );
  assert_eq!(db.get("src/a.md", "en").await.unwrap(), lines(&["hello"]));
  assert_eq!(db.get("src/b.md", "zh").await.unwrap(), lines(&["第二个"]));
  assert!(db.get("src/a.md", "ja").await.unwrap().is_empty());
  assert!(db.get("nope.md", "zh").await.unwrap().is_empty());
}

#[compio::test]
async fn last_write_wins() {
  let dir = tempfile::tempdir().unwrap();
  let db = Db::open(db_path(&dir)).unwrap();

  db.set("a.md", "zh", &["old1", "old2"]).await.unwrap();
  db.set("a.md", "zh", &["new"]).await.unwrap();
  assert_eq!(db.get("a.md", "zh").await.unwrap(), lines(&["new"]));

  // 空内容也是有效的覆盖
  db.set("a.md", "zh", &[] as &[&str]).await.unwrap();
  assert!(db.get("a.md", "zh").await.unwrap().is_empty());
}

#[compio::test]
async fn prefix_lines_roundtrip() {
  let dir = tempfile::tempdir().unwrap();
  let db = Db::open(db_path(&dir)).unwrap();

  let content = ["|开头的行", "@开头的行", "", " 空格 "];
  db.set("a.md", "zh", &content).await.unwrap();
  assert_eq!(db.get("a.md", "zh").await.unwrap(), lines(&content));
}

#[compio::test]
async fn garbage_lines_ignored() {
  let dir = tempfile::tempdir().unwrap();
  let path = db_path(&dir);
  fs::write(&path, "@a.md/zh\r\n# 注释\r\n|第一行\r\n乱码\r\n|第二行").unwrap();

  let db = Db::open(&path).unwrap();
  assert_eq!(
    db.get("a.md", "zh").await.unwrap(),
    lines(&["第一行", "第二行"])
  );
}

#[compio::test]
async fn long_line_cross_buffer() {
  let dir = tempfile::tempdir().unwrap();
  let db = Db::open(db_path(&dir)).unwrap();
  let long = "x".repeat(200 * 1024);
  let content = vec![long.clone(), "tail".to_owned()];
  db.set("big.md", "zh", &content).await.unwrap();
  assert_eq!(db.get("big.md", "zh").await.unwrap(), content);
}

#[compio::test]
async fn invalid_input_rejected() {
  let dir = tempfile::tempdir().unwrap();
  let db = Db::open(db_path(&dir)).unwrap();

  assert!(matches!(
    db.set("", "zh", &["x"]).await,
    Err(Error::EmptyPath)
  ));
  assert!(matches!(
    db.set("a", "", &["x"]).await,
    Err(Error::EmptyLang)
  ));
  assert!(matches!(
    db.set("a", "z/h", &["x"]).await,
    Err(Error::BadLang(_))
  ));
  assert!(matches!(
    db.set("a\nb", "zh", &["x"]).await,
    Err(Error::BadPath(_))
  ));
  assert!(matches!(
    db.set("a", "zh", &["x\ny"]).await,
    Err(Error::BadLine)
  ));
}

#[compio::test]
async fn compact_all_sorts_and_dedupes() {
  let dir = tempfile::tempdir().unwrap();
  let db = Db::open(db_path(&dir)).unwrap();

  db.set("b.md", "zh", &["b-old"]).await.unwrap();
  db.set("a.md", "zh", &["a-old"]).await.unwrap();
  db.set("b.md", "zh", &["b-new"]).await.unwrap();
  db.set("a.md", "en", &["a-en"]).await.unwrap();

  db.compact(&["a.md", "b.md"]).await.unwrap();

  let text = fs::read_to_string(db.path()).unwrap();
  assert_eq!(
    text,
    "@a.md/en\n|a-en\n@a.md/zh\n|a-old\n@b.md/zh\n|b-new\n"
  );
  assert_eq!(db.get("b.md", "zh").await.unwrap(), lines(&["b-new"]));

  // 压实后继续追加仍可用
  db.set("c.md", "zh", &["c"]).await.unwrap();
  assert_eq!(db.get("c.md", "zh").await.unwrap(), lines(&["c"]));
}

#[compio::test]
async fn compact_subset_drops_others() {
  let dir = tempfile::tempdir().unwrap();
  let db = Db::open(db_path(&dir)).unwrap();

  db.set("a.md", "zh", &["a1"]).await.unwrap();
  db.set("z.md", "zh", &["z1"]).await.unwrap();
  db.set("a.md", "zh", &["a2"]).await.unwrap();
  db.set("z.md", "zh", &["z2"]).await.unwrap();

  db.compact(&["a.md"]).await.unwrap();

  let text = fs::read_to_string(db.path()).unwrap();
  // 只保留白名单路径，z.md 全部删除
  assert_eq!(text, "@a.md/zh\n|a2\n");
  assert_eq!(db.get("a.md", "zh").await.unwrap(), lines(&["a2"]));
  assert!(db.get("z.md", "zh").await.unwrap().is_empty());
}

#[compio::test]
async fn compact_many_records_cross_buffer() {
  let dir = tempfile::tempdir().unwrap();
  let db = Db::open(db_path(&dir)).unwrap();

  let path_li: Vec<String> = (0..2000).map(|i| format!("dir{i:04}.md")).collect();
  // 足够多的记录，确保扫描跨越多个 64 KiB 读缓冲
  for i in (0..2000).rev() {
    let path = format!("dir{i:04}.md");
    db.set(&path, "zh", &["旧"]).await.unwrap();
  }
  for i in 0..2000 {
    let path = format!("dir{i:04}.md");
    db.set(&path, "zh", &["新"]).await.unwrap();
  }
  db.compact(&path_li).await.unwrap();

  let text = fs::read_to_string(db.path()).unwrap();
  let head_li: Vec<&str> = text.lines().filter(|l| l.starts_with('@')).collect();
  assert_eq!(head_li.len(), 2000);
  assert_eq!(head_li[0], "@dir0000.md/zh");
  assert_eq!(head_li[1999], "@dir1999.md/zh");
  assert!(text.contains("|新") && !text.contains("|旧"));
  assert_eq!(db.get("dir1999.md", "zh").await.unwrap(), lines(&["新"]));
}

#[compio::test]
async fn compact_missing_file_is_noop() {
  let dir = tempfile::tempdir().unwrap();
  let db = Db::open(db_path(&dir)).unwrap();
  db.compact::<&str>(&[]).await.unwrap();
  assert!(!db.path().exists());
}

#[compio::test]
async fn compact_empty_clears_db() {
  let dir = tempfile::tempdir().unwrap();
  let db = Db::open(db_path(&dir)).unwrap();

  db.set("a.md", "zh", &["a"]).await.unwrap();
  db.set("b.md", "zh", &["b"]).await.unwrap();
  db.compact::<&str>(&[]).await.unwrap();

  assert_eq!(fs::read_to_string(db.path()).unwrap(), "");
  assert!(db.get("a.md", "zh").await.unwrap().is_empty());
  assert!(db.get("b.md", "zh").await.unwrap().is_empty());
}

#[compio::test]
async fn concurrent_sets_do_not_interleave() {
  let dir = tempfile::tempdir().unwrap();
  let db = Rc::new(Db::open(db_path(&dir)).unwrap());

  let task_li: Vec<_> = (0..32)
    .map(|i| {
      let db = Rc::clone(&db);
      spawn(async move {
        let path = format!("k{i}.md");
        db.set(&path, "zh", &["第一行", "第二行"]).await.unwrap();
      })
    })
    .collect();
  for task in task_li {
    task.await.unwrap();
  }

  for i in 0..32 {
    let path = format!("k{i}.md");
    assert_eq!(
      db.get(&path, "zh").await.unwrap(),
      lines(&["第一行", "第二行"])
    );
  }
}

#[compio::test]
async fn second_instance_rejected() {
  let dir = tempfile::tempdir().unwrap();
  let path = db_path(&dir);

  let db = Db::open(&path).unwrap();
  db.set("a.md", "zh", &["x"]).await.unwrap();
  assert!(matches!(Db::open(&path), Err(Error::Locked(_))));

  // 释放后可以重新打开
  drop(db);
  let db = Db::open(&path).unwrap();
  assert_eq!(db.get("a.md", "zh").await.unwrap(), lines(&["x"]));
}
