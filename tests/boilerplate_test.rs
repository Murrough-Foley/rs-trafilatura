use rs_trafilatura::extract;

#[test]
fn nav_is_excluded_even_inside_article() {
    let html = r#"
        <html>
          <body>
            <article>
              <nav>MENU_TEXT</nav>
              <p>BODY_TEXT</p>
            </article>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("BODY_TEXT"));
            assert!(!result.content_text.contains("MENU_TEXT"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn site_footer_is_excluded_but_article_footer_is_preserved() {
    let html = r#"
        <html>
          <body>
            <footer>SITE_FOOTER_TEXT</footer>
            <article>
              <p>ARTICLE_BODY</p>
              <footer>ARTICLE_FOOTER_TEXT</footer>
            </article>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("ARTICLE_BODY"));
            assert!(result.content_text.contains("ARTICLE_FOOTER_TEXT"));
            assert!(!result.content_text.contains("SITE_FOOTER_TEXT"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn aside_is_excluded_even_inside_article() {
    let html = r#"
        <html>
          <body>
            <article>
              <aside>RELATED_SIDEBAR_TEXT</aside>
              <p>ARTICLE_BODY</p>
            </article>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("ARTICLE_BODY"));
            assert!(!result.content_text.contains("RELATED_SIDEBAR_TEXT"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn related_and_recommended_sections_are_excluded_by_class() {
    let html = r#"
        <html>
          <body>
            <article>
              <p>ARTICLE_BODY</p>
              <div class="recommended">RECOMMENDED_TEXT</div>
              <div class="more-from">MORE_FROM_TEXT</div>
              <div class="you-may-like">YOU_MAY_LIKE_TEXT</div>
            </article>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("ARTICLE_BODY"));
            assert!(!result.content_text.contains("RECOMMENDED_TEXT"));
            assert!(!result.content_text.contains("MORE_FROM_TEXT"));
            assert!(!result.content_text.contains("YOU_MAY_LIKE_TEXT"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn footer_related_legal_classes_are_excluded_by_class() {
    let html = r#"
        <html>
          <body>
            <article>
              <p>ARTICLE_BODY</p>
              <div class="copyright">COPYRIGHT_TEXT</div>
              <div class="legal">LEGAL_TEXT</div>
              <div class="disclaimer">DISCLAIMER_TEXT</div>
              <div class="site-footer">SITE_FOOTER_TEXT</div>
            </article>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("ARTICLE_BODY"));
            assert!(!result.content_text.contains("COPYRIGHT_TEXT"));
            assert!(!result.content_text.contains("LEGAL_TEXT"));
            assert!(!result.content_text.contains("DISCLAIMER_TEXT"));
            assert!(!result.content_text.contains("SITE_FOOTER_TEXT"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn schema_org_breadcrumb_list_inside_article_is_excluded() {
    let html = r#"
        <html>
          <body>
            <article>
              <ol itemscope itemtype="https://schema.org/BreadcrumbList">
                <li>Home</li>
                <li>Section</li>
              </ol>
              <p>BODY_TEXT</p>
            </article>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("BODY_TEXT"));
            assert!(!result.content_text.contains("Home"));
            assert!(!result.content_text.contains("Section"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn site_header_is_excluded_but_article_header_is_preserved() {
    let html = r#"
        <html>
          <body>
            <header>
              <nav>SITE_NAV_TEXT</nav>
            </header>
            <article>
              <header>
                <h1>ARTICLE_HEADER_TITLE</h1>
              </header>
              <p>ARTICLE_BODY</p>
            </article>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(
                result.content_text.contains("ARTICLE_HEADER_TITLE"),
                "content_text={}",
                result.content_text
            );
            assert!(result.content_text.contains("ARTICLE_BODY"));
            assert!(!result.content_text.contains("SITE_NAV_TEXT"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn navbar_class_is_excluded() {
    let html = r#"
        <html>
          <body>
            <div class="navbar">NAVBAR_TEXT</div>
            <article><p>BODY_TEXT</p></article>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("BODY_TEXT"));
            assert!(!result.content_text.contains("NAVBAR_TEXT"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn breadcrumb_class_is_excluded() {
    let html = r#"
        <html>
          <body>
            <nav class="breadcrumb">Home / Section</nav>
            <article><p>BODY_TEXT</p></article>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("BODY_TEXT"));
            assert!(!result.content_text.contains("Home"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn schema_org_breadcrumb_list_is_excluded() {
    let html = r#"
        <html>
          <body>
            <ol itemscope itemtype="https://schema.org/BreadcrumbList">
              <li>Home</li>
              <li>Section</li>
            </ol>
            <article><p>BODY_TEXT</p></article>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("BODY_TEXT"));
            assert!(!result.content_text.contains("Home"));
            assert!(!result.content_text.contains("Section"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn top_nav_class_is_excluded() {
    let html = r#"
        <html>
          <body>
            <div class="top-nav">TOP_NAV_TEXT</div>
            <article><p>BODY_TEXT</p></article>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("BODY_TEXT"));
            assert!(!result.content_text.contains("TOP_NAV_TEXT"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn main_menu_class_is_excluded() {
    let html = r#"
        <html>
          <body>
            <ul class="main-menu">MAIN_MENU_TEXT</ul>
            <article><p>BODY_TEXT</p></article>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("BODY_TEXT"));
            assert!(!result.content_text.contains("MAIN_MENU_TEXT"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn site_nav_class_is_excluded() {
    let html = r#"
        <html>
          <body>
            <div class="site_nav">SITE_NAV_TEXT</div>
            <article><p>BODY_TEXT</p></article>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("BODY_TEXT"));
            assert!(!result.content_text.contains("SITE_NAV_TEXT"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn header_inside_main_is_preserved() {
    let html = r#"
        <html>
          <body>
            <header>SITE_HEADER_TEXT</header>
            <main>
              <header>
                <h1>MAIN_HEADER_TITLE</h1>
              </header>
              <p>MAIN_BODY</p>
            </main>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("MAIN_HEADER_TITLE"));
            assert!(result.content_text.contains("MAIN_BODY"));
            assert!(!result.content_text.contains("SITE_HEADER_TEXT"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn ad_class_is_excluded() {
    let html = r#"
        <html>
          <body>
            <article>
              <p>BODY_TEXT</p>
              <div class="ad">AD_TEXT</div>
            </article>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("BODY_TEXT"));
            assert!(!result.content_text.contains("AD_TEXT"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn sponsored_class_is_excluded() {
    let html = r#"
        <html>
          <body>
            <article>
              <p>BODY_TEXT</p>
              <div class="sponsored">SPONSORED_TEXT</div>
            </article>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("BODY_TEXT"));
            assert!(!result.content_text.contains("SPONSORED_TEXT"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn ad_id_is_excluded() {
    let html = r#"
        <html>
          <body>
            <article>
              <p>BODY_TEXT</p>
              <div id="google_ads">GOOGLE_ADS_TEXT</div>
            </article>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("BODY_TEXT"));
            assert!(!result.content_text.contains("GOOGLE_ADS_TEXT"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn ins_tag_is_excluded() {
    let html = r#"
        <html>
          <body>
            <article>
              <p>BODY_TEXT</p>
              <ins>INS_AD_TEXT</ins>
            </article>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("BODY_TEXT"));
            assert!(!result.content_text.contains("INS_AD_TEXT"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn advertisement_class_is_excluded() {
    let html = r#"
        <html>
          <body>
            <article>
              <p>BODY_TEXT</p>
              <div class="advertisement">ADVERTISEMENT_TEXT</div>
            </article>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("BODY_TEXT"));
            assert!(!result.content_text.contains("ADVERTISEMENT_TEXT"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn banner_ad_id_is_excluded() {
    let html = r#"
        <html>
          <body>
            <article>
              <p>BODY_TEXT</p>
              <div id="banner-ad">BANNER_AD_TEXT</div>
            </article>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("BODY_TEXT"));
            assert!(!result.content_text.contains("BANNER_AD_TEXT"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn address_class_is_not_treated_as_ad() {
    let html = r#"
        <html>
          <body>
            <article>
              <div class="address">ADDRESS_TEXT</div>
              <p>BODY_TEXT</p>
            </article>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("ADDRESS_TEXT"));
            assert!(result.content_text.contains("BODY_TEXT"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn share_buttons_are_excluded() {
    let html = r#"
        <html>
          <body>
            <article>
              <p>BODY_TEXT</p>
              <div class="share-buttons">SHARE_TEXT</div>
            </article>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("BODY_TEXT"));
            assert!(!result.content_text.contains("SHARE_TEXT"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn social_widget_is_excluded() {
    let html = r#"
        <html>
          <body>
            <article>
              <p>BODY_TEXT</p>
              <div class="social">SOCIAL_TEXT</div>
            </article>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("BODY_TEXT"));
            assert!(!result.content_text.contains("SOCIAL_TEXT"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn footer_inside_main_is_preserved() {
    let html = r#"
        <html>
          <body>
            <footer>SITE_FOOTER_TEXT</footer>
            <main>
              <p>MAIN_BODY</p>
              <footer>MAIN_FOOTER_TEXT</footer>
            </main>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("MAIN_BODY"));
            assert!(result.content_text.contains("MAIN_FOOTER_TEXT"));
            assert!(!result.content_text.contains("SITE_FOOTER_TEXT"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn aside_inside_main_is_excluded() {
    let html = r#"
        <html>
          <body>
            <main>
              <aside>SIDEBAR_TEXT</aside>
              <p>MAIN_BODY</p>
            </main>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("MAIN_BODY"));
            assert!(!result.content_text.contains("SIDEBAR_TEXT"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}

#[test]
fn nested_aside_is_excluded() {
    let html = r#"
        <html>
          <body>
            <article>
              <p>BODY_TEXT</p>
              <aside>
                <div class="widget">
                  <aside>NESTED_ASIDE_TEXT</aside>
                </div>
              </aside>
            </article>
          </body>
        </html>
    "#;

    let result = extract(html);
    match result {
        Ok(result) => {
            assert!(result.content_text.contains("BODY_TEXT"));
            assert!(!result.content_text.contains("NESTED_ASIDE_TEXT"));
        }
        Err(err) => panic!("expected Ok(_), got Err({err:?})"),
    }
}
