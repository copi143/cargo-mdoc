# cargo-mdoc 内联紧凑代码格式 (Compact Code Format)

本文档描述了一种专为在纯文本环境（如下游的 JSON 字段中）高效存储带有高亮（语法分析）信息的 Rust 源代码而设计的**内联长度前缀编码**格式。

## 1. 设计目标

1. **极高压缩率**：避免传统的 JSON AST 结构 `{ "type": "keyword", "value": "pub" }` 带来的严重体积膨胀。
2. **绝对安全的内容逃逸**：无需引入复杂的转义规则（如 `"` 或 `\`），任何奇特的源码内容（包括用于格式控制的特殊字符自身）都不会破坏解析。
3. **极简解析**：前端可以利用 O(n) 的单次遍历游标快速将其还原为可渲染的 Token 序列。

## 2. 格式规范

该格式由一系列连续的**编码单元 (Encoded Unit)** 组成。没有任何全局分隔符。

每个编码单元的结构如下：
`[长度(可选)][类型/控制符][内容(仅普通Token)]`

### 2.1 长度前缀 (Length)

* **规则**：由 0 到 9 的 ASCII 数字组成。
* **省略**：如果长度为 `1`，则**必须省略**数字前缀。
* **单位极度重要**：长度的单位必须是 **UTF-16 Code Unit**。例如，普通的英文字母长度为 1，大部分中文字符长度为 1，但 Emoji（如 🦀）长度为 2。这确保了 Web 前端（JavaScript）使用 `substring` 切片时分毫不差。

### 2.2 控制符单元 (Control Characters)

用于压缩代码中最常见的大片空白。如果类型标识是控制符，则该单元**不包含后续内容块**，解析器应直接生成指定长度的空白。

| 控制符 | 含义           | 解析规则                           |
|:-------|:---------------|:-----------------------------------|
| `+`    | 空格 (Space)   | 生成指定长度连续的 ` ` (ASCII 32)  |
| `$`    | 换行 (Newline) | 生成指定长度连续的 `\n` (ASCII 10) |

*注意：所有回车符 `\r` 必须在编码前被丢弃/规整为 `\n`。所有制表符 `\t` 必须在编码前被展开为相应数量的空格（推荐展开为 4 个）。*

### 2.3 Token 类型符 (Token Type)

代表具体的语法类别。由单字符（通常为小写英文字母）表示。如果读取到 Token 类型符，解析器必须盲目向后截取精确对应长度的字符作为内容。

| 类型符 | 分类名称                          | 代表示例 (Rust)                    |
|:-------|:----------------------------------|:-----------------------------------|
| `k`    | Keyword (关键字)                  | `pub`, `fn`, `mut`, `impl`, `let`  |
| `t`    | Type (类型)                       | `String`, `Option`, `Self`, `u32`  |
| `f`    | Function (函数名)                 | `main`, `clone`, `new`             |
| `v`    | Variable/Identifier (变量/标识符) | 各种普通变量名、模块名             |
| `m`    | Macro (宏)                        | `println!`, `vec!` (包含感叹号)    |
| `s`    | String (字符串字面量)             | `"hello"`, `r#"raw"#`              |
| `n`    | Number (数字字面量)               | `10`, `0x1A`, `1.5f32`             |
| `c`    | Comment (注释)                    | `// text`, `/* block */`           |
| `l`    | Lifetime (生命周期)               | `'a`, `'static`                    |
| `o`    | Operator (操作符)                 | `+`, `-`, `*`, `>`, `->`, `=>`     |
| `p`    | Punctuation (标点符号)            | `{`, `}`, `(`, `)`, `;`, `,`, `:`  |
| `a`    | Attribute (属性)                  | `#[derive(Debug)]`, `#![no_std]`   |
| `_`    | Default (无色文本/未分类)         | 未命中的所有其他合法 Rust 源码文本 |

### 2.4 极端示例解析

**示例代码：**

```rust
pub fn main() {
    let a = 0;
}
```

**对应编码流：**
`3kpub+2kfn+4fmainp(p)+p{$4+3klet+vav=n0p;$p}`

**拆解步骤：**

1. `3kpub`: 数字3 -> 截取3位 -> 读类型'k' -> 盲切3位内容 "pub"
2. `+`: 无数字(len=1) -> 读类型'+'(空格控制符) -> 生成 " "
3. `2kfn`: 数字2 -> 截取2位 -> 读类型'k' -> 盲切2位内容 "fn"
4. `+`: 无数字(len=1) -> 生成 " "
5. `4fmain`: 数字4 -> 类型'f' -> 截取 "main"
6. `p(`: 无数字(len=1) -> 类型'p' -> 截取 "("
7. `p)`: 无数字(len=1) -> 类型'p' -> 截取 ")"
8. `+`: 无数字(len=1) -> 生成 " "
9. `p{`: 无数字(len=1) -> 类型'p' -> 截取 "{"
10. `$`: 无数字(len=1) -> 类型'$'(换行控制符) -> 生成 "
"
11. `4+`: 数字4 -> 类型'+' -> 生成 "    " (4个空格)
12. `3klet`: 数字3 -> 类型'k' -> 截取 "let"
13. `+`: 无数字(len=1) -> 生成 " "
14. `v`: **特殊：无数字(len=1)** -> 类型'v' -> 盲切1位内容 "a" (原本为 `vav`，即 1位长类型v内容为a，后跟一个字母v实际上会被误读为类型v... 等等！)

**⚠️ 重要边缘情况修复：单字母内容冲突**
如果在上述流中遇到 `vav`，第一位是隐式长度1，第二位类型`v`，第三位盲切1位内容`a`。那么紧随其后的 `=` 如果前缀是无数字类型，会变成 `o=`。
即：`va+o=...` (变量a，空格，操作符=)

如果需要表示单字符 `a` 类型为 `v`，编码就是 `va`。
如果需要表示单字符 `0` 类型为 `n`，编码就是 `n0`。
这是绝对安全的，因为前面只要有字母（类型符），解析器就必然进入“盲切文本”阶段，此时截取出来的内容不会被当作新的前缀处理。

## 3. 实现指南 (Rust 编码端)

在 `cargo-mdoc` 的处理流中，当遍历 `ra_ap_syntax` 生成的 Token 序列时，遵守以下法则：

1. **收集并规整**：
    从 AST 提取 Text 时，立即将所有 `
` 丢弃。将所有 ` ` 替换为 4 个空格字符 ` `。
2. **长度计算**：
    使用 UTF-16 Code Unit 计算长度：`let len = text.encode_utf16().count();`
3. **合并相邻控制符**：
    如果有连续的空白，建议合并生成以提高压缩率。例如，遇到一个空格再遇到三个空格，应当直接合并输出 `4+`。
4. **字符串格式化**：

    ```rust
    // 伪代码示例
    if token_is_space {
        if len == 1 { write!("+"); } else { write!("{}+", len); }
    } else if token_is_newline {
         if len == 1 { write!("$"); } else { write!("{}$", len); }
    } else {
        if len == 1 {
            write!("{}{}", type_char, text);
        } else {
            write!("{}{}{}", len, type_char, text);
        }
    }
    ```

## 4. Web 前端解析示例 (JavaScript)

```javascript
/**
 * @param {string} encodedStr
 * @returns {Array<{type: string, text: string}>}
 */
export function decodeCompactCode(encodedStr) {
    const tokens = [];
    let i = 0;
    
    while (i < encodedStr.length) {
        // 1. 读取可选的长度前缀
        let lenStr = "";
        while (i < encodedStr.length && encodedStr[i] >= '0' && encodedStr[i] <= '9') {
            lenStr += encodedStr[i];
            i++;
        }
        const len = lenStr === "" ? 1 : parseInt(lenStr, 10);
        
        // 2. 读取类型标识
        const typeChar = encodedStr[i++];
        
        // 3. 处理快捷控制符
        if (typeChar === '+') {
            tokens.push({ type: 'space', text: ' '.repeat(len) });
            continue;
        }
        if (typeChar === '$') {
            tokens.push({ type: 'newline', text: '
'.repeat(len) });
            continue;
        }
        
        // 4. 处理常规 Token（盲切）
        const text = encodedStr.substring(i, i + len);
        tokens.push({ type: typeChar, text: text });
        
        i += len;
    }
    
    return tokens;
}
```
