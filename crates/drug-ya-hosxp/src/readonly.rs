//! Defense-in-depth SQL guard.
//!
//! This is **not** the security boundary - DB user grants and session
//! read-only mode are. This layer exists to catch mistakes early: any
//! outgoing statement that is not a single `SELECT`/`WITH` is rejected.

/// Statement executed on every new pooled connection to make the session
/// reject any DML.
pub const READ_ONLY_SESSION_SQL: &str = "SET SESSION TRANSACTION READ ONLY";

/// The guard rejected a statement.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("rejected non-read-only SQL statement")]
pub struct GuardError;

/// Accepts only a single read statement, case-insensitively, after leading
/// SQL comments. Anything else - `INSERT`, `UPDATE`, `DELETE`, DDL, multiple
/// statements - is rejected.
pub fn assert_read_only(sql: &str) -> Result<(), GuardError> {
    let Some(rest) = strip_leading_comments(sql) else {
        return Err(GuardError);
    };
    let rest = rest.trim_start();
    let keyword_end = rest
        .find(|c: char| !c.is_ascii_alphanumeric())
        .unwrap_or(rest.len());
    let keyword = rest[..keyword_end].to_ascii_uppercase();
    let followed_by_boundary = rest[keyword_end..]
        .chars()
        .next()
        .is_some_and(|c| c.is_whitespace() || c == '(');
    let single_statement = !has_semicolon_outside_quotes(rest);

    if (keyword == "SELECT" || keyword == "WITH") && followed_by_boundary && single_statement {
        Ok(())
    } else {
        Err(GuardError)
    }
}

fn has_semicolon_outside_quotes(sql: &str) -> bool {
    let mut quote = None;
    for c in sql.chars() {
        if let Some(active) = quote {
            if c == active {
                quote = None;
            }
        } else if matches!(c, '\'' | '"' | '`') {
            quote = Some(c);
        } else if c == ';' {
            return true;
        }
    }
    false
}

fn strip_leading_comments(mut sql: &str) -> Option<&str> {
    loop {
        sql = sql.trim_start();
        if let Some(rest) = sql.strip_prefix("--") {
            sql = rest.split_once('\n').map(|(_, after)| after).unwrap_or("");
        } else if let Some(rest) = sql.strip_prefix("/*") {
            sql = &rest[rest.find("*/")? + 2..];
        } else {
            return Some(sql);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_select_variants() {
        assert!(assert_read_only("SELECT 1").is_ok());
        assert!(assert_read_only("select * from opitemrece").is_ok());
        assert!(assert_read_only("  SELECT icode FROM drugitems").is_ok());
        assert!(assert_read_only("SELECT\nvstdate FROM opitemrece").is_ok());
        assert!(assert_read_only("SELECT(1)").is_ok());
    }

    #[test]
    fn accepts_with_variants() {
        assert!(assert_read_only("WITH x AS (SELECT 1) SELECT * FROM x").is_ok());
    }

    #[test]
    fn accepts_statements_with_leading_comments() {
        assert!(assert_read_only("-- generated query\nSELECT 1").is_ok());
        assert!(assert_read_only("/* header */ SELECT 1").is_ok());
    }

    #[test]
    fn rejects_write_and_ddl_statements() {
        for sql in [
            "INSERT INTO opitemrece (icode) VALUES ('1001')",
            "UPDATE opitemrece SET qty = 0",
            "DELETE FROM opitemrece",
            "DROP TABLE opitemrece",
            "ALTER TABLE opitemrece ADD COLUMN x INT",
            "CREATE TABLE x (id INT)",
            "GRANT SELECT ON *.* TO 'u'",
            "TRUNCATE TABLE opitemrece",
        ] {
            assert!(assert_read_only(sql).is_err(), "should reject: {sql}");
        }
    }

    #[test]
    fn rejects_multiple_statements_in_one_string() {
        assert!(assert_read_only("SELECT 1; DELETE FROM opitemrece").is_err());
        assert!(assert_read_only("SELECT 1;").is_err());
    }

    #[test]
    fn rejects_keyword_prefix_masquerading_as_select() {
        assert!(assert_read_only("SELECTED 1").is_err());
    }

    #[test]
    fn rejects_empty_and_unterminated_comment() {
        assert!(assert_read_only("").is_err());
        assert!(assert_read_only("/* never closed SELECT 1").is_err());
    }
}
