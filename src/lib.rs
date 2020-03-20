use std::collections::HashMap;
use std::cell::RefCell;
use js_sys::Uint8Array;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response, MouseEvent, HtmlAnchorElement, Document, HtmlElement, HtmlSelectElement, HtmlInputElement, HtmlCanvasElement, CanvasRenderingContext2d};
use bincode::{deserialize, serialize};
use base64::encode;

struct AppData{
    canvas: HtmlCanvasElement,
    context: CanvasRenderingContext2d,
    select: HtmlSelectElement,
    select_strokes: HtmlSelectElement,
    select_points: HtmlSelectElement,
    search: HtmlInputElement,
    add: HtmlInputElement,
    document: Document,
    point: Option<(i32, i32)>,
    chars: Vec<char>,
    strokes: HashMap<char, Vec<Vec<(u16, u16)>>>
}

thread_local!{
    static APP_DATA: RefCell<AppData> = RefCell::new({
        let document = web_sys::window().unwrap().document().unwrap();
        AppData{
            document: document.clone(),
            canvas: {
                let canvas = document.get_element_by_id("canvas").unwrap();
                canvas.dyn_into::<HtmlCanvasElement>()
                    .map_err(|_| ())
                    .unwrap()
            },
            select: {
                let element = document.get_element_by_id("select").unwrap();
                element.dyn_into::<HtmlSelectElement>()
                    .map_err(|_| ())
                    .unwrap()
            },
            select_strokes: {
                let element = document.get_element_by_id("select_strokes").unwrap();
                element.dyn_into::<HtmlSelectElement>()
                    .map_err(|_| ())
                    .unwrap()
            },
            select_points: {
                let element = document.get_element_by_id("select_points").unwrap();
                element.dyn_into::<HtmlSelectElement>()
                    .map_err(|_| ())
                    .unwrap()
            },
            search: {
                let element = document.get_element_by_id("search").unwrap();
                element.dyn_into::<HtmlInputElement>()
                    .map_err(|_| ())
                    .unwrap()
            },
            add: {
                let element = document.get_element_by_id("txt_add").unwrap();
                element.dyn_into::<HtmlInputElement>()
                    .map_err(|_| ())
                    .unwrap()
            },
            context: {
                let canvas = document.get_element_by_id("canvas").unwrap();
                let canvas: HtmlCanvasElement = canvas
                    .dyn_into::<HtmlCanvasElement>()
                    .map_err(|_| ())
                    .unwrap();
                canvas
                .get_context("2d")
                .unwrap()
                .unwrap()
                .dyn_into::<CanvasRenderingContext2d>()
                .unwrap()
            },
            point: None,
            chars: vec![],
            strokes: HashMap::new()
        }
    });
}

fn start() -> Result<JsValue, JsValue> {
    log("start.");
    APP_DATA.with(|app_data| -> Result<JsValue, JsValue> {
        let app_data = app_data.borrow();
        //设置字体
        app_data.context.set_font("800px 楷体_GB2312");
        app_data.context.set_stroke_style(&JsValue::from_str("#000"));
        app_data.context.set_line_width(6.0);
        
        //点击设置替换点
        let closure = Closure::wrap(Box::new(move |event: MouseEvent| {
            APP_DATA.with(|app_data| -> Result<JsValue, JsValue>{
                let mut app_data = app_data.borrow_mut();
                let x = event.offset_x() * 2;
                let y = event.offset_y() * 2;
                app_data.point = Some((x, y));
                let ch = app_data.select.value();
                draw_ch(&*app_data, ch, false, false)
            }).expect("字符绘制失败");
        }) as Box<dyn FnMut(_)>);
        app_data.canvas.add_event_listener_with_callback("mousedown", closure.as_ref().unchecked_ref())?;
        closure.forget();
        
        //点击切换字符
        let on_select_change = Closure::wrap(Box::new(move |_e: HtmlSelectElement| {
            APP_DATA.with(|app_data| -> Result<JsValue, JsValue>{
                let app_data = app_data.borrow();
                let ch = app_data.select.value();
                draw_ch(&*app_data, ch, true, true)
            }).expect("字符绘制失败");
        }) as Box<dyn FnMut(_)>);
        app_data.select.set_onchange(Some(on_select_change.as_ref().unchecked_ref()));
        on_select_change.forget();

        //点击切换笔画
        let on_select_stroke_change = Closure::wrap(Box::new(move |_e: HtmlSelectElement| {
            APP_DATA.with(|app_data| -> Result<JsValue, JsValue>{
                let app_data = app_data.borrow();
                let ch = app_data.select.value();
                draw_ch(&*app_data, ch, false, true)
            }).expect("字符绘制失败");
        }) as Box<dyn FnMut(_)>);
        app_data.select_strokes.set_onchange(Some(on_select_stroke_change.as_ref().unchecked_ref()));
        on_select_stroke_change.forget();
        
        //点击切换笔画对应的点
        let on_select_points_change = Closure::wrap(Box::new(move |_e: HtmlSelectElement| {
            APP_DATA.with(|app_data| -> Result<JsValue, JsValue>{
                let app_data = app_data.borrow();
                let ch = app_data.select.value();
                draw_ch(&*app_data, ch, false, false)
            }).expect("字符绘制失败");
        }) as Box<dyn FnMut(_)>);
        app_data.select_points.set_onchange(Some(on_select_points_change.as_ref().unchecked_ref()));
        on_select_points_change.forget();
        
        //添加所有字符
        for ch in app_data.chars.iter() {
            let option = app_data.document.create_element("option")?;
            option.set_text_content(Some(&format!("{}", ch)));
            app_data.select.append_child(&option)?;
        }
        app_data.select.set_selected_index(0);
        draw_ch(&*app_data, app_data.select.value(), true, true)?;
        app_data.select.set_value(&app_data.select.value());

        //搜索
        let on_search_change = Closure::wrap(Box::new(move |_e: HtmlSelectElement| {
            APP_DATA.with(|app_data| -> Result<JsValue, JsValue>{
                let app_data = app_data.borrow();
                let ch = app_data.search.value();
                if ch.len() == 0 {
                    return Ok(JsValue::FALSE);
                }
                if let Ok(idx) = app_data.chars.binary_search(&ch.chars().next().unwrap()){
                    app_data.select.set_selected_index(idx as i32);
                    draw_ch(&*app_data, ch, true, true)?;
                }else{
                    alert("没有这个字!");
                }
                Ok(JsValue::TRUE)
            }).expect("字符搜索失败");
        }) as Box<dyn FnMut(_)>);
        app_data.search.set_onchange(Some(on_search_change.as_ref().unchecked_ref()));
        on_search_change.forget();

        Ok(JsValue::TRUE)
    })?;
    
    let btn_add_click = Closure::wrap(Box::new(move |_e: HtmlSelectElement| {
        APP_DATA.with(|app_data| -> Result<JsValue, JsValue>{
            let mut app_data = app_data.borrow_mut();
            //添加字符
            let ch = app_data.add.value();
            let ch = ch.trim();
            if ch.len() == 0 {
                alert("请输入字符!");
                return Ok(JsValue::FALSE);
            }
            let chr = ch.chars().nth(0).unwrap();

            let idx = if let Ok(idx) = app_data.chars.binary_search(&chr){
                alert("字符已存在!");
                idx
            }else{
                app_data.chars.push(chr);
                app_data.strokes.insert(chr,  vec![vec![(50,50)]]);

                //添加所有字符
                app_data.select.set_text_content(None);
                for ch in app_data.chars.iter() {
                    let option = app_data.document.create_element("option")?;
                    option.set_text_content(Some(&format!("{}", ch)));
                    app_data.select.append_child(&option)?;
                }

                app_data.chars.len()-1
            };
            //选择对应的字符
            app_data.select.set_selected_index(idx as i32);
            draw_ch(&*app_data, ch.to_string(), true, true)
        }).expect("字符绘制失败");
    }) as Box<dyn FnMut(_)>);
    get_element_by_id("btn_add").set_onclick(Some(btn_add_click.as_ref().unchecked_ref()));
    btn_add_click.forget();

    let btn_replace_click = Closure::wrap(Box::new(move |_e: HtmlElement| {
        change_point(0).expect("change_point调用失败");
    }) as Box<dyn FnMut(_)>);
    get_element_by_id("btn_replace").set_onclick(Some(btn_replace_click.as_ref().unchecked_ref()));
    btn_replace_click.forget();

    let btn_insert_before_click = Closure::wrap(Box::new(move |_e: HtmlElement| {
        change_point(1).expect("change_point调用失败");
    }) as Box<dyn FnMut(_)>);
    get_element_by_id("btn_insert_before").set_onclick(Some(btn_insert_before_click.as_ref().unchecked_ref()));
    btn_insert_before_click.forget();

    let btn_insert_after_click = Closure::wrap(Box::new(move |_e: HtmlElement| {
        change_point(2).expect("change_point调用失败");
    }) as Box<dyn FnMut(_)>);
    get_element_by_id("btn_insert_after").set_onclick(Some(btn_insert_after_click.as_ref().unchecked_ref()));
    btn_insert_after_click.forget();

    let btn_delete_click = Closure::wrap(Box::new(move |_e: HtmlElement| {
        change_point(3).expect("change_point调用失败");
    }) as Box<dyn FnMut(_)>);
    get_element_by_id("btn_delete").set_onclick(Some(btn_delete_click.as_ref().unchecked_ref()));
    btn_delete_click.forget();

    let btn_add_stroke_click = Closure::wrap(Box::new(move || {
        APP_DATA.with(|app_data| -> Result<JsValue, JsValue>{
            let mut app_data = app_data.borrow_mut();
            //添加一笔
            let ch = app_data.select.value();
            let key = ch.chars().next().unwrap();
            let strokes = app_data.strokes.get_mut(&key).unwrap();
            strokes.push(vec![(50,50)]);
            draw_ch(&*app_data, ch, true, true)
        }).expect("add_stroke调用失败");
    }) as Box<dyn FnMut()>);
    get_element_by_id("btn_add_stroke").set_onclick(Some(btn_add_stroke_click.as_ref().unchecked_ref()));
    btn_add_stroke_click.forget();

    let btn_delete_stroke_click = Closure::wrap(Box::new(move || {
        APP_DATA.with(|app_data| -> Result<JsValue, JsValue>{
            let mut app_data = app_data.borrow_mut();
            //删除一笔
            let ch = app_data.select.value().chars().next().unwrap();
            let select_index = app_data.select_strokes.selected_index() as usize;
            let strokes = app_data.strokes.get_mut(&ch).unwrap();
            strokes.remove(select_index);
            draw_ch(&*app_data, format!("{}", ch), true, true)
        }).expect("delete_stroke调用失败");
    }) as Box<dyn FnMut()>);
    get_element_by_id("btn_delete_stroke").set_onclick(Some(btn_delete_stroke_click.as_ref().unchecked_ref()));
    btn_delete_stroke_click.forget();

    let btn_move_forward_click = Closure::wrap(Box::new(move || {
        chagne_stroke(0, None).expect("move_forward调用失败");
    }) as Box<dyn FnMut()>);
    get_element_by_id("btn_move_forward").set_onclick(Some(btn_move_forward_click.as_ref().unchecked_ref()));
    btn_move_forward_click.forget();

    let btn_move_backward = Closure::wrap(Box::new(move || {
        chagne_stroke(1, None).expect("move_backward调用失败");
    }) as Box<dyn FnMut()>);
    get_element_by_id("btn_move_backward").set_onclick(Some(btn_move_backward.as_ref().unchecked_ref()));
    btn_move_backward.forget();

    let btn_move_backward2_click =Closure::wrap(Box::new(move || {
        chagne_stroke(2, None).expect("move_backward2调用失败");
    }) as Box<dyn FnMut()>);
    get_element_by_id("btn_move_backward2").set_onclick(Some(btn_move_backward2_click.as_ref().unchecked_ref()));
    btn_move_backward2_click.forget();

    let btn_move_backward4_click = Closure::wrap(Box::new(move || {
        //2笔移动到最后
        chagne_stroke(2, Some(0)).expect("move_backward4调用失败");
    }) as Box<dyn FnMut()>);
    get_element_by_id("btn_move_backward4").set_onclick(Some(btn_move_backward4_click.as_ref().unchecked_ref()));
    btn_move_backward4_click.forget();

    let btn_move_backward3_click = Closure::wrap(Box::new(move || {
        //后移3笔
        chagne_stroke(3, None).expect("move_backward3调用失败");
    }) as Box<dyn FnMut()>);
    get_element_by_id("btn_move_backward3").set_onclick(Some(btn_move_backward3_click.as_ref().unchecked_ref()));
    btn_move_backward3_click.forget();

    let btn_move_backward5_click = Closure::wrap(Box::new(move || {
        //3笔移动到最后
        chagne_stroke(3, Some(0)).expect("move_backward5调用失败");
    }) as Box<dyn FnMut()>);
    get_element_by_id("btn_move_backward5").set_onclick(Some(btn_move_backward5_click.as_ref().unchecked_ref()));
    btn_move_backward5_click.forget();

    let gen_vec_click = Closure::wrap(Box::new(move || {
        gen_vec().expect("gen_vec调用失败");
    }) as Box<dyn FnMut()>);
    get_element_by_id("gen_vec").set_onclick(Some(gen_vec_click.as_ref().unchecked_ref()));
    gen_vec_click.forget();

    let gen_map_click = Closure::wrap(Box::new(move || {
        gen_map().expect("gen_map调用失败");
    }) as Box<dyn FnMut()>);
    get_element_by_id("gen_map").set_onclick(Some(gen_map_click.as_ref().unchecked_ref()));
    gen_map_click.forget();

    Ok(JsValue::TRUE)
}

fn get_element_by_id(id:&str) -> HtmlElement{
    web_sys::window().unwrap().document().unwrap()
                .get_element_by_id(id).unwrap()
                .dyn_into::<HtmlElement>()
                .map_err(|_| ())
                .unwrap()
}

fn draw_ch(app_data:&AppData, ch: String, reset_strokes: bool, reset_points: bool) -> Result<JsValue, JsValue> {
    app_data.search.set_value(&ch);
    app_data.context.set_fill_style(&JsValue::from_str("#777"));
    //let ch = SELECT.value().unwrap();
    let (width, height) = (app_data.canvas.width() as f64, app_data.canvas.height() as f64);
    app_data.context.clear_rect(0.0, 0.0, width, height);
    app_data.context.fill_text(&ch, width * 0.1, height * 0.75)?;

    let key = ch.chars().next().unwrap();

    let map = &app_data.strokes;
    let strokes = map.get(&key).unwrap();
    if reset_strokes {
        //创建笔画数据
        app_data.select_strokes.set_text_content(None);
        for (id, stroke) in strokes.iter().enumerate() {
            let option = app_data.document.create_element("option")?;
            option.set_text_content(Some(&format!("{}:{}点", id + 1, stroke.len())));
            app_data.select_strokes.append_child(&option)?;
        }
        app_data.select_strokes.set_selected_index(0);
    }

    if reset_points{
        //清空对应的所有点
        app_data.select_points.set_text_content(None);
        let idx = app_data.select_strokes.selected_index() as usize;
        for point in strokes[idx].iter() {
            let option = app_data.document.create_element("option")?;
            option.set_text_content(Some(&format!("({},{})", point.0, point.1)));
            app_data.select_points.append_child(&option)?;
        }
        app_data.select_points.set_selected_index(0);
    }

    //绘制所有笔画
    draw_strokes(app_data, key);

    //绘制笔画当前选择的点
    let idx = app_data.select_strokes.selected_index() as usize;
    let pt = strokes[idx][app_data.select_strokes.selected_index() as usize];
    app_data.context.set_fill_style(&JsValue::from_str("#f00"));
    app_data.context.begin_path();
    app_data.context.arc(pt.0 as f64, pt.1 as f64, 20.0, 0.0, 360.0)?;
    app_data.context.fill();

    //绘制用户点击的点
    if let Some(point) = app_data.point.as_ref() {
        app_data.context.set_fill_style(&JsValue::from_str("rgba(0, 0, 255, 0.5)"));
        app_data.context.begin_path();
        app_data.context.arc(point.0 as f64, point.1 as f64, 20.0, 0.0, 360.0)?;
        app_data.context.fill();

        app_data.context.set_fill_style(&JsValue::from_str("rgba(255, 255, 0, 0.5)"));
        app_data.context.begin_path();
        app_data.context.arc(point.0 as f64, point.1 as f64, 10.0, 0.0, 360.0)?;
        app_data.context.fill();
    }

    Ok(JsValue::TRUE)
}

fn draw_strokes(app_data:&AppData, ch: char) {
    let map = &app_data.strokes;
    let strokes = map.get(&ch).unwrap();
    let select_stroke = app_data.select_strokes.selected_index() as usize;
    for (i, stroke) in strokes.iter().enumerate() {
        //当前笔画红色
        if i == select_stroke {
            app_data.context.set_stroke_style(&JsValue::from_str("#f00"));
        } else {
            app_data.context.set_stroke_style(&JsValue::from_str("#000"));
        }
        app_data.context.begin_path();
        app_data.context.move_to(stroke[0].0 as f64, stroke[0].1 as f64);
        for i in 1..stroke.len() {
            app_data.context.line_to(stroke[i].0 as f64, stroke[i].1 as f64);
        }
        app_data.context.stroke();
    }
}

fn chagne_stroke(op:i32, val:Option<i32>) -> Result<JsValue, JsValue>{
    hide_download();
    APP_DATA.with(|app_data| -> Result<JsValue, JsValue>{
        let mut app_data = app_data.borrow_mut();
        //替换当前字符
        let ch = app_data.select.value().chars().next().unwrap();
        //获取所有笔画
        let select_index = app_data.select_strokes.selected_index() as usize;
        let strokes = app_data.strokes.get_mut(&ch).unwrap();

        if op==0{
            //前移笔画
            if select_index>0{
                let before = strokes[select_index-1].clone();
                strokes[select_index-1] = strokes[select_index].clone();
                strokes[select_index] = before;
                app_data.select_strokes.set_selected_index(select_index as i32-1);
                draw_ch(&*app_data, app_data.select.value(), false, true)?;
            }else{
                alert("已经是第一笔了!");
            }
        }else if op==1{
            //后移笔画
            if select_index<strokes.len()-1{
                let after = strokes[select_index+1].clone();
                strokes[select_index+1] = strokes[select_index].clone();
                strokes[select_index] = after;
                app_data.select_strokes.set_selected_index(select_index as i32+1);
                draw_ch(&*app_data, app_data.select.value(), false, true)?;
            }else{
                alert("已经到最后一笔了!");
            }
        }else if op==2 || op ==3{
            //（当前）两笔/三笔后移
            let count = op as usize;
            let index = select_index;

            if strokes.len()<=count || strokes.len()-index<count{
                alert("长度不够！");
                return Ok(JsValue::FALSE);
            }
            if strokes.len()-index==count{
                alert("已经到最后了！");
                return Ok(JsValue::FALSE);
            }

            let mut insert_index = index+1;
            if val.is_some(){
                //移动到最后
                insert_index = strokes.len()-count;  
            }

            //删除当前N笔
            let movestrokes:Vec<Vec<(u16, u16)>> = strokes.splice(index..index+count, [].iter().cloned()).collect();
            //重新插入N笔
            for s in movestrokes.iter().rev(){
                strokes.insert(insert_index, s.clone());
            }
            if val.is_some(){
                app_data.select_strokes.set_selected_index(insert_index as i32);
            }else{
                app_data.select_strokes.set_selected_index(select_index as i32+1);
            }
            draw_ch(&*app_data, app_data.select.value(), false, true)?;
        }

        Ok(JsValue::TRUE)
    })
}

fn change_point(op:i32) -> Result<JsValue, JsValue>{
    hide_download();
    APP_DATA.with(|app_data| -> Result<JsValue, JsValue>{
        let mut app_data = app_data.borrow_mut();
        let point = app_data.point;
        //替换当前字符
        let ch = app_data.select.value().chars().next().unwrap();
        //获取所有笔画
        let select_index = app_data.select_strokes.selected_index() as usize;
        let strokes = app_data.strokes.get_mut(&ch).unwrap();
        //获取选择的笔画
        let points:&mut Vec<(u16, u16)> = &mut strokes[select_index];
        if points.len() == 0{
            return Ok(JsValue::FALSE);
        }
        let mut point_index = select_index;
        if op==3{
            //只有一个点不删除
            if points.len()==1{
                alert("只有一个点了!");
                return Ok(JsValue::FALSE);
            }
            //删除当前点
            points.remove(point_index);
            if point_index==points.len(){
                point_index -=1;
            }
        }else{
            if let Some(point) = point {
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
        let ch = app_data.select.value();
        draw_ch(&*app_data, ch, false, true)?;
        //选中编辑的点
        let ch = app_data.select.value();
        app_data.select_points.set_selected_index(point_index as i32);
        draw_ch(&*app_data, ch, false, false)
    })
}

//生成map数据
fn gen_map() -> Result<JsValue, JsValue> {
    APP_DATA.with(|app_data| -> Result<JsValue, JsValue>{
        let app_data = app_data.borrow();
        //序列化
        let data: Vec<u8> = serialize(&app_data.strokes).unwrap();
        let link_em = get_element_by_id("download_button");
        link_em.set_attribute("download", "gb2312.data")?;
        let link = link_em.dyn_into::<HtmlAnchorElement>()
                    .map_err(|_| ())
                    .unwrap();
        link.set_href(&format!(r#"data:application/octet-stream;base64,{}"#, encode(&data)));
        link.set_inner_text("gb2312.data");
        Ok(JsValue::TRUE)
    })
}

//生成数组
fn gen_vec() -> Result<JsValue, JsValue> {
    APP_DATA.with(|app_data| -> Result<JsValue, JsValue>{
        let app_data = app_data.borrow();
        //序列化
        let mut vec:Vec<(char, Vec<Vec<(u16, u16)>>)> = vec![];

        for ch in app_data.chars.iter(){
            vec.push((*ch, app_data.strokes.get(ch).unwrap().clone()));
        }

        let data: Vec<u8> = serialize(&vec).unwrap();
        let link_em = get_element_by_id("download_button");
        link_em.set_attribute("download", "STROKES.data")?;
        let link = link_em.dyn_into::<HtmlAnchorElement>()
                    .map_err(|_| ())
                    .unwrap();
        link.set_href(&format!(r#"data:application/octet-stream;base64,{}"#, encode(&data)));
        link.set_inner_text("STROKES.data(替换页面中STROKES.data)");
        Ok(JsValue::TRUE)
    })
}

fn hide_download(){
    let link_em = get_element_by_id("download_button");
    let link = link_em.dyn_into::<HtmlAnchorElement>()
                .map_err(|_| ())
                .unwrap();
    link.set_href("");
    link.set_inner_text("");
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
    fn alert(s: &str);
}

#[wasm_bindgen]
pub async fn run() -> Result<JsValue, JsValue> {
    //加载文件
    let mut opts = RequestInit::new();
    opts.method("GET");
    opts.mode(RequestMode::Cors);

    let request = Request::new_with_str_and_init("STROKES.data", &opts)?;
    let window = web_sys::window().unwrap();
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;

    let resp: Response = resp_value.dyn_into().unwrap();
    let buffer = JsFuture::from(resp.array_buffer()?).await?;
    let data:Vec<u8> = Uint8Array::new(&buffer).to_vec();
    let strokes: Vec<(char, Vec<Vec<(u16, u16)>>)> = deserialize(&data).unwrap();
    log(&format!("字符个数{:?}", strokes.len()));

    let mut count_map = HashMap::new();

    let mut chars = vec![];
    for (ch, _strokes) in &strokes {
        chars.push(*ch);
        if count_map.contains_key(ch){
            *count_map.get_mut(ch).unwrap() += 1;
        }else{
            count_map.insert(*ch, 1);
        }
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

    APP_DATA.with(|app_data|{
        let mut app_data = app_data.borrow_mut();
        app_data.chars = chars;
        let mut map = HashMap::new();
        for (ch, strokes) in strokes {
            map.insert(ch, strokes);
        }
        app_data.strokes = map;
    });
    start()
}