extern crate bincode;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate stdweb;
extern crate bzip2;
use bzip2::Compression;
use std::io::prelude::*;
use std::collections::HashMap;

use bincode::{deserialize, serialize};
use std::sync::Mutex;
use stdweb::unstable::TryInto;
use stdweb::web::event::*;
use stdweb::web::html_element::*;
use stdweb::web::*;

lazy_static! {
    static ref XML_HTTP_REQUEST:XmlHttpRequest = {
        js!(
            var oReq = new XMLHttpRequest();
            oReq.responseType = "arraybuffer";
            return oReq
        ).try_into().unwrap()
    };
    //要插入的点
    static ref POINT: Mutex<Option<(f64, f64)>> = Mutex::new(None);
    static ref CONTEXT: CanvasRenderingContext2d = {
        let canvas: CanvasElement = document()
            .get_element_by_id("canvas")
            .unwrap()
            .try_into()
            .unwrap();
        canvas.get_context().unwrap()
    };
    static ref CANVAS: CanvasElement = document()
        .get_element_by_id("canvas")
        .unwrap()
        .try_into()
        .unwrap();
    static ref SELECT: SelectElement = document()
        .get_element_by_id("select")
        .unwrap()
        .try_into()
        .unwrap();
    static ref SELECT_STROKES: SelectElement = document()
        .get_element_by_id("select_strokes")
        .unwrap()
        .try_into()
        .unwrap();
    static ref SELECT_POINTS: SelectElement = document()
        .get_element_by_id("select_points")
        .unwrap()
        .try_into()
        .unwrap();
    static ref SEARCH: InputElement = document()
        .get_element_by_id("search")
        .unwrap()
        .try_into()
        .unwrap();
    static ref ADD: InputElement = document()
        .get_element_by_id("txt_add")
        .unwrap()
        .try_into()
        .unwrap();
    static ref CHARS: Mutex<Vec<char>> = Mutex::new(vec![]);
    static ref STROKES: Mutex<HashMap<char, Vec<Vec<(u16, u16)>>>> = Mutex::new(HashMap::new());
}

fn draw_ch(ch: String, reset_strokes: bool, reset_points: bool) {
    SEARCH.set_raw_value(&ch);
    CONTEXT.set_fill_style_color("#777");
    //let ch = SELECT.value().unwrap();
    let (width, height) = (CANVAS.width() as f64, CANVAS.height() as f64);
    CONTEXT.clear_rect(0.0, 0.0, width, height);
    CONTEXT.fill_text(&ch, width * 0.1, height * 0.75, None);

    let key = ch.chars().next().unwrap();

    let map = STROKES.lock().unwrap();
    let strokes = map.get(&key).unwrap();
    if reset_strokes {
        //创建笔画数据
        SELECT_STROKES.set_text_content("");
        for (id, stroke) in strokes.iter().enumerate() {
            let option = document().create_element("option").unwrap();
            option.set_text_content(&format!("{}:{}点", id + 1, stroke.len()));
            SELECT_STROKES.append_child(&option);
        }
        SELECT_STROKES.set_selected_index(Some(0));
    }

    if reset_points{
        //清空对应的所有点
        SELECT_POINTS.set_text_content("");
        let idx = SELECT_STROKES.selected_index().unwrap() as usize;
        for point in strokes[idx].iter() {
            let option = document().create_element("option").unwrap();
            option.set_text_content(&format!("({},{})", point.0, point.1));
            SELECT_POINTS.append_child(&option);
        }
        SELECT_POINTS.set_selected_index(Some(0));
    }

    //绘制所有笔画
    draw_strokes(key);

    //绘制笔画当前选择的点
    let idx = SELECT_STROKES.selected_index().unwrap() as usize;
    let pt = strokes[idx][SELECT_POINTS.selected_index().unwrap() as usize];
    CONTEXT.set_fill_style_color("#f00");
    CONTEXT.begin_path();
    CONTEXT.arc(pt.0 as f64, pt.1 as f64, 20.0, 0.0, 360.0, false);
    CONTEXT.fill(FillRule::NonZero);

    //绘制用户点击的点
    if let Some(point) = *POINT.lock().unwrap() {
        CONTEXT.set_fill_style_color("rgba(0, 0, 255, 0.5)");
        CONTEXT.begin_path();
        CONTEXT.arc(point.0 as f64, point.1 as f64, 20.0, 0.0, 360.0, false);
        CONTEXT.fill(FillRule::NonZero);

        CONTEXT.set_fill_style_color("rgba(255, 255, 0, 0.5)");
        CONTEXT.begin_path();
        CONTEXT.arc(point.0 as f64, point.1 as f64, 10.0, 0.0, 360.0, false);
        CONTEXT.fill(FillRule::NonZero);
    }
}

fn draw_strokes(ch: char) {
    let map = STROKES.lock().unwrap();
    let strokes = map.get(&ch).unwrap();
    let select_stroke = SELECT_STROKES.selected_index().unwrap() as usize;
    for (i, stroke) in strokes.iter().enumerate() {
        //当前笔画红色
        if i == select_stroke {
            CONTEXT.set_stroke_style_color("#f00");
        } else {
            CONTEXT.set_stroke_style_color("#000");
        }
        CONTEXT.begin_path();
        CONTEXT.move_to(stroke[0].0 as f64, stroke[0].1 as f64);
        for i in 1..stroke.len() {
            CONTEXT.line_to(stroke[i].0 as f64, stroke[i].1 as f64);
        }
        CONTEXT.stroke();
    }
}

fn chagne_stroke(op:i32){
    //替换当前字符
    let ch = SELECT.value().unwrap().chars().next().unwrap();
    //获取所有笔画
    let mut map = STROKES.lock().unwrap();
    let strokes = map.get_mut(&ch).unwrap();
    let select_index = SELECT_STROKES.selected_index().unwrap() as usize;

    if op==0{
        //前移笔画
        if select_index>0{
            let before = strokes[select_index-1].clone();
            strokes[select_index-1] = strokes[select_index].clone();
            strokes[select_index] = before;
            draw_ch(SELECT.value().unwrap(), true, true);
            SELECT_STROKES.set_selected_index(Some(select_index as u32-1));
        }
    }else if op==1{
        //后移笔画
        if select_index<strokes.len()-1{
            let after = strokes[select_index+1].clone();
            strokes[select_index+1] = strokes[select_index].clone();
            strokes[select_index] = after;
            draw_ch(SELECT.value().unwrap(), true, true);
            SELECT_STROKES.set_selected_index(Some(select_index as u32+1));
        }
    }
}

fn change_point(op:i32){
    //替换当前字符
    let ch = SELECT.value().unwrap().chars().next().unwrap();
    //获取所有笔画
    let mut map = STROKES.lock().unwrap();
    let strokes = map.get_mut(&ch).unwrap();
    let select_index = SELECT_STROKES.selected_index().unwrap() as usize;
    //获取选择的笔画
    let points:&mut Vec<(u16, u16)> = &mut strokes[select_index];
    if points.len() == 0{
        return;
    }
    let mut point_index = SELECT_POINTS.selected_index().unwrap() as usize;
    if op==3{
        //只有一个点不删除
        if points.len()==1{
            js!(alert("只有一个点了!"));
            return;
        }
        //删除当前点
        points.remove(point_index);
        if point_index==points.len(){
            point_index -=1;
        }
    }else{
        if let Some(point) = *POINT.lock().unwrap() {
            if op == 0{
                //替换当前点
                points[point_index] = (point.0 as u16, point.1 as u16);
            }else if op == 1{
                //在前边插入点
                points.insert(point_index, (point.0 as u16, point.1 as u16));
            }else if op==2{
                //在后边插入点
                points.insert(point_index+1, (point.0 as u16, point.1 as u16));
                point_index += 1;
            }   
        }
    }
    draw_ch(SELECT.value().unwrap(), false, true);
    //选中编辑的点
    SELECT_POINTS.set_selected_index(Some(point_index as u32));
    draw_ch(SELECT.value().unwrap(), false, false);
}

//生成压缩数据
fn gen_map_bzip2() {
    //序列化
    let encoded: Vec<u8> = serialize(&*STROKES.lock().unwrap()).unwrap();
    //压缩
    let mut zip = bzip2::write::BzEncoder::new(vec![], Compression::Best);
    zip.write_all(&encoded).unwrap();
    let data = zip.finish().unwrap();
    js!{
        var url = URL.createObjectURL(new File([new Uint8Array(@{data})], "gb2312.data"));
        window.open(url);
        window.URL.revokeObjectURL(url);
    }
}

//生成map数据
fn gen_map() {
    //序列化
    let data: Vec<u8> = serialize(&*STROKES.lock().unwrap()).unwrap();
    js!{
        var url = URL.createObjectURL(new File([new Uint8Array(@{data})], "gb2312.data"));
        window.open(url);
        window.URL.revokeObjectURL(url);
    }
}

//生成数组
fn gen_vec() {
    //序列化
    let mut vec:Vec<(char, Vec<Vec<(u16, u16)>>)> = vec![];

    let strokes = STROKES.lock().unwrap();
    for ch in CHARS.lock().unwrap().iter(){
        vec.push((*ch, strokes.get(ch).unwrap().clone()));
    }

    let data: Vec<u8> = serialize(&vec).unwrap();
    js!{
        //var blob = new Blob([new Uint8Array(@{data})], {type: "application/octet-stream"});
        var url = URL.createObjectURL(new File([new Uint8Array(@{data})], "STROKES.data"));
        window.open(url);
        window.URL.revokeObjectURL(url);
    }
}

fn main() {
    stdweb::initialize();

    //加载文件
    XML_HTTP_REQUEST.open("GET", "STROKES.data").unwrap();
    XML_HTTP_REQUEST.add_event_listener(move |_event:ReadyStateChangeEvent|{
        if XML_HTTP_REQUEST.ready_state() == XhrReadyState::Done {
            let array_buffer:ArrayBuffer = XML_HTTP_REQUEST.raw_response().try_into().unwrap();
            let data:Vec<u8> = Vec::from(array_buffer);
            let strokes: Vec<(char, Vec<Vec<(u16, u16)>>)> = deserialize(&data).unwrap();
            js!(console.log("字符个数", @{strokes.len() as i32}));
            let mut chars = vec![];
            for (ch, _strokes) in &strokes {
                chars.push(*ch);
            }

            //检查是否有不存在的字
            //let articls: HashMap<String, String> = deserialize(&ARTICLS).unwrap();
            // let poems: HashMap<String, String> = deserialize(&POEMS).unwrap();

            // for (_title, content) in &poems {
            //     for ch in content.chars(){
            //         match ch {
            //             '—' => continue,
            //             '。' => continue,
            //             '，' => continue,
            //             '；' => continue,
            //             '”' => continue,
            //             '“' => continue,
            //             '‘' => continue,
            //             '’' => continue,
            //             '！' => continue,
            //             '：' => continue,
            //             '？' => continue,
            //             '!'|'0'|'1'|'2'|'3'|'4'|'5'|'6'|'7'|'8'|'9'|']'|'['|'、'|','|':'|'…'|'·'|'《'|'》'|'?'|'`'|'"'|' '|'［'|'］' => continue,
            //             _ => (),
            //         }
            //         if !chars.contains(&ch){
            //             js!(console.log("没有文字:", @{format!("{}", ch)}));
            //         }
            //     }
            // }

            *CHARS.lock().unwrap() = chars;

            let mut map = HashMap::new();
            for (ch, strokes) in strokes {
                map.insert(ch, strokes);
            }
            *STROKES.lock().unwrap() = map;
            start();
        }
    });
    XML_HTTP_REQUEST.send().unwrap();
    stdweb::event_loop();
}

fn start(){
    //设置字体
    CONTEXT.set_font("800px 楷体_GB2312");
    CONTEXT.set_stroke_style_color("#000");
    CONTEXT.set_line_width(6.0);

    //点击设置替换点
    CANVAS.add_event_listener(|event: PointerDownEvent| {
        let x = event.offset_x() * 2.0;
        let y = event.offset_y() * 2.0;
        *POINT.lock().unwrap() = Some((x, y));
        draw_ch(SELECT.value().unwrap(), false, false);
    });

    //点击切换字符
    SELECT.add_event_listener(|_: ChangeEvent| {
        draw_ch(SELECT.value().unwrap(), true, true);
    });

    //点击切换笔画
    SELECT_STROKES.add_event_listener(|_: ChangeEvent| {
        //draw_strokes(SELECT.value().unwrap().chars().next().unwrap());
        draw_ch(SELECT.value().unwrap(), false, true);
    });

    //点击切换笔画对应的点
    SELECT_POINTS.add_event_listener(|_: ChangeEvent| {
        //刷新
        draw_ch(SELECT.value().unwrap(), false, false);
    });

    document()
        .get_element_by_id("btn_add")
        .unwrap()
        .add_event_listener(|_: ClickEvent| {
            //添加字符
            let ch = ADD.raw_value();
            let ch = ch.trim();
            if ch.len() == 0 {
                js!(alert("请输入字符!"));
                return;
            }
            let chr = ch.chars().nth(0).unwrap();
            let mut chars = CHARS.lock().unwrap();

            let idx = if let Ok(idx) = chars.binary_search(&chr){
                js!(alert("字符已存在!"));
                idx
            }else{
                chars.push(chr);
                let mut strokes = STROKES.lock().unwrap();
                strokes.insert(chr,  vec![vec![(50,50)]]);

                //添加所有字符
                SELECT.set_text_content("");
                for ch in chars.iter() {
                    let option = document().create_element("option").unwrap();
                    option.set_text_content(&format!("{}", ch));
                    SELECT.append_child(&option);
                }

                chars.len()-1
            };
            //选择对应的字符
            SELECT.set_selected_index(Some(
                idx as u32,
            ));
            draw_ch(String::from(ch), true, true);
        });

    document()
        .get_element_by_id("btn_replace")
        .unwrap()
        .add_event_listener(|_: ClickEvent| {
            change_point(0);
        });

    document()
        .get_element_by_id("btn_insert_before")
        .unwrap()
        .add_event_listener(|_: ClickEvent| {
            change_point(1);
        });

    document()
        .get_element_by_id("btn_insert_after")
        .unwrap()
        .add_event_listener(|_: ClickEvent| {
            change_point(2);
        });

    document()
        .get_element_by_id("btn_delete")
        .unwrap()
        .add_event_listener(|_: ClickEvent| {
            change_point(3);
        });

    document()
        .get_element_by_id("btn_add_stroke")
        .unwrap()
        .add_event_listener(|_: ClickEvent| {
            //添加一笔
            let ch = SELECT.value().unwrap();
            let key = ch.chars().next().unwrap();
            let mut map = STROKES.lock().unwrap();
            let strokes = map.get_mut(&key).unwrap();
            strokes.push(vec![(50,50)]);
            draw_ch(String::from(ch), true, true);
        });

    document()
        .get_element_by_id("btn_move_forward")
        .unwrap()
        .add_event_listener(|_: ClickEvent| {
            //前移笔画
            chagne_stroke(0);
        });

    document()
        .get_element_by_id("btn_move_backward")
        .unwrap()
        .add_event_listener(|_: ClickEvent| {
            //后移笔画
            chagne_stroke(1);
        });

    document()
    .get_element_by_id("gen_map_bzip2")
    .unwrap()
    .add_event_listener(|_: ClickEvent| {
        gen_map_bzip2();
    });

    document()
    .get_element_by_id("gen_vec")
    .unwrap()
    .add_event_listener(|_: ClickEvent| {
        gen_vec();
    });

    document()
    .get_element_by_id("gen_map")
    .unwrap()
    .add_event_listener(|_: ClickEvent| {
        gen_map();
    });

    // document()
    // .get_element_by_id("choose_file")
    // .unwrap()
    // .add_event_listener(|event: DataTransfer| {
    //     js!(console.log(@{event.files()}));
    // });

    //添加所有字符
    let chars = CHARS.lock().unwrap();
    for ch in chars.iter() {
        let option = document().create_element("option").unwrap();
        option.set_text_content(&format!("{}", ch));
        SELECT.append_child(&option);
    }
    SELECT.set_selected_index(Some(0));
    draw_ch(SELECT.value().unwrap(), true, true);
    SELECT.set_raw_value(&SELECT.value().unwrap());

    SEARCH.add_event_listener(move |_: ChangeEvent| {
        let ch = SEARCH.raw_value();
        if ch.len() == 0 {
            return;
        }
        if let Ok(idx) = chars.binary_search(&ch.chars().next().unwrap()){
            SELECT.set_selected_index(Some(
                idx as u32,
            ));
            draw_ch(ch, true, true);
        }else{
            js!(alert("没有这个字!"));
        }
    });
}
