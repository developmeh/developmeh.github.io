//! The user-agent stylesheet: a compact rendition of the HTML spec's
//! "Rendering" section restricted to the properties of plan §5.
//!
//! Chromium's defaults are followed where they matter for the visual diff
//! (serif body font, 8px body margin, heading sizes, 40px list indents).

/// UA stylesheet source.
pub const UA_STYLESHEET: &str = r#"
:root { font-family: serif; font-size: 16px; color: black; }
html, body { display: block; }
body { margin: 8px; }

head, meta, title, link, style, script, template, base, area, basefont,
datalist, noframes, param, rp, [hidden], input[type=hidden i] { display: none; }

address, blockquote, center, div, figure, figcaption, footer, form, header,
hr, legend, listing, main, p, plaintext, pre, xmp, article, aside, h1, h2,
h3, h4, h5, h6, hgroup, nav, section, dir, dd, dl, dt, menu, ol, ul,
fieldset, details, summary, optgroup, option, search, dialog { display: block; }

li { display: list-item; }

table { display: table; box-sizing: border-box; border-spacing: 2px; border-color: gray; text-indent: initial; }
caption { display: table-caption; text-align: center; }
tr { display: table-row; vertical-align: middle; }
thead { display: table-header-group; vertical-align: middle; }
tbody { display: table-row-group; vertical-align: middle; }
tfoot { display: table-footer-group; vertical-align: middle; }
td, th { display: table-cell; padding: 1px; vertical-align: middle; }
th { font-weight: bold; text-align: center; }

blockquote, figure { margin: 1em 40px; }
p, dl { margin: 1em 0; }
dd { margin-left: 40px; }
ol, ul, menu, dir { margin: 1em 0; padding-left: 40px; }
ul, menu, dir { list-style-type: disc; }
ol { list-style-type: decimal; }
ol ul, ul ul, menu ul, dir ul { list-style-type: circle; }
ol ol ul, ol ul ul, ul ol ul, ul ul ul { list-style-type: square; }
ul ul, ul ol, ol ul, ol ol, menu menu, dir dir { margin-top: 0; margin-bottom: 0; }

h1 { font-size: 2em; margin: 0.67em 0; font-weight: bold; }
h2 { font-size: 1.5em; margin: 0.83em 0; font-weight: bold; }
h3 { font-size: 1.17em; margin: 1em 0; font-weight: bold; }
h4 { font-size: 1em; margin: 1.33em 0; font-weight: bold; }
h5 { font-size: 0.83em; margin: 1.67em 0; font-weight: bold; }
h6 { font-size: 0.67em; margin: 2.33em 0; font-weight: bold; }
article h1, aside h1, nav h1, section h1 { font-size: 1.5em; margin: 0.83em 0; }

pre, listing, plaintext, xmp { font-family: monospace; white-space: pre; margin: 1em 0; }
code, kbd, samp, tt, var { font-family: monospace; }
textarea { white-space: pre-wrap; }

b, strong { font-weight: bold; }
i, em, cite, var, address, dfn { font-style: italic; }
u, ins { text-decoration: underline; }
s, strike, del { text-decoration: line-through; }
abbr[title], acronym[title] { text-decoration: underline; }
small { font-size: smaller; }
big { font-size: larger; }
sub, sup { font-size: smaller; line-height: normal; }
sub { vertical-align: bottom; }
sup { vertical-align: top; }
mark { background-color: yellow; color: black; }

a:link, a:any-link { color: #0000ee; text-decoration: underline; }
a:link:active { color: #ff0000; }

hr { color: gray; border-style: solid; border-width: 1px; margin: 0.5em auto; }
fieldset { margin: 0 2px; padding: 0.35em 0.75em 0.625em; border: 2px solid #c0c0c0; }
legend { padding: 0 2px; }

center { text-align: center; }
img { display: inline-block; }
svg { display: inline-block; }
iframe { display: inline-block; border: 2px solid gray; width: 300px; height: 150px; }
video, audio, canvas, object, embed { display: inline-block; }
video { width: 300px; height: 150px; }
canvas { width: 300px; height: 150px; }

input, button, select, textarea, meter, progress { display: inline-block; font-family: sans-serif; font-size: 13.3333px; }
input, textarea, select { border: 2px solid #767676; padding: 1px 2px; color: black; background-color: white; }
textarea { width: 20em; height: 2em; }
button, input[type=submit i], input[type=button i], input[type=reset i] { border: 2px solid #767676; padding: 1px 6px; background-color: #efefef; color: black; text-align: center; }
input[type=checkbox i], input[type=radio i] { width: 13px; height: 13px; padding: 0; border: 1px solid #767676; margin: 3px 3px 3px 4px; }
button { display: inline-block; }

details > summary:first-of-type { display: list-item; }
dialog:not([open]) { display: none; }

br { display: inline; }
wbr { display: inline; }
ruby { display: inline; }
rt { font-size: 50%; }
"#;
