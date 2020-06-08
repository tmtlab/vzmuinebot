/* ===============================================================================
Бот для сбора меню у рестораторов и выдача их желающим покушать.
Обработка диалога редактирования блюда ресторана. 03 June 2020.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use teloxide::{
    prelude::*,
    types::{InputFile, ReplyMarkup},
};

use crate::commands as cmd;
use crate::database as db;
use crate::eater;
use crate::caterer;
use crate::cat_group;

// Показывает информацию о блюде 
//
pub async fn next_with_info(cx: cmd::Cx<(i32, i32)>) -> cmd::Res {
    // Извлечём параметры
    let (rest_id, dish_id) = cx.dialogue;
    
   // Получаем информацию из БД
   let (info, dish_image_id) = match db::dish_info(rest_id, dish_id).await {
      Some(dish_info) => dish_info,
      None => (format!("Ошибка db::dish_info({})", rest_id), None)
   };

    // Отображаем информацию о блюде и оставляем кнопки главного меню. Если для блюда задана картинка, то текст будет комментарием
    if let Some(image_id) = dish_image_id {
        // Создадим графический объект
        let image = InputFile::file_id(image_id);

        // Отправляем картинку и текст как комментарий
        cx.answer_photo(image)
        .caption(info)
        .reply_markup(ReplyMarkup::ReplyKeyboardMarkup(cmd::Caterer::main_menu_markup()))
        .send()
        .await?;
    } else {
        cx.answer(info)
        .reply_markup(cmd::Caterer::main_menu_markup())
        .send()
        .await?;
    }

    // Переходим (остаёмся) в режим редактирования блюда
    next(cmd::Dialogue::CatEditDish(rest_id, dish_id))
}

async fn next_with_cancel(cx: cmd::Cx<(i32, i32)>, text: &str) -> cmd::Res {
    cx.answer(text)
    .reply_markup(cmd::Caterer::main_menu_markup())
    .send()
    .await?;

    // Извлечём параметры
    let (rest_id, dish_id) = cx.dialogue;

    // Остаёмся в режиме редактирования блюда
    next(cmd::Dialogue::CatEditDish(rest_id, dish_id))
}


// Режим редактирования у ресторана rest_id группы group_id
pub async fn edit_dish_mode(cx: cmd::Cx<(i32, i32)>) -> cmd::Res {
     
    // Извлечём параметры
    let (rest_id, dish_id) = cx.dialogue;
    
    // Разбираем команду.
     match cx.update.text() {
        None => {
            cx.answer("Текстовое сообщение, пожалуйста!").send().await?;

            // Остаёмся в режиме редактирования блюда
            next(cmd::Dialogue::CatEditDish(rest_id, dish_id))
        }
        Some(command) => {
            match cmd::CatDish::from(rest_id, dish_id, command) {

                 // Показать информацию о ресторане (возврат в главное меню ресторатора)
                 cmd::CatDish::Main(rest_id) => {
                    // Покажем информацию
                    let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
                    caterer::next_with_info(DialogueDispatcherHandlerCx::new(bot, update, rest_id), false).await
                }

                // Выйти из режима ресторатора
                cmd::CatDish::Exit => {
                    let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
                    eater::start(DialogueDispatcherHandlerCx::new(bot, update, ())).await
                }

               // Изменение названия блюда
                cmd::CatDish::EditTitle(rest_id, dish_id) => {

                    // Отправляем приглашение ввести строку со слешем в меню для отмены
                    cx.answer(format!("Введите название (/ для отмены)"))
                    .reply_markup(cmd::Caterer::slash_markup())
                    .send()
                    .await?;

                    // Переходим в режим ввода нового названия
                    next(cmd::Dialogue::CatEditDishTitle(rest_id, dish_id))
                }

                // Изменение информации о блюде
                cmd::CatDish::EditInfo(rest_id, dish_id) => {

                    // Отправляем приглашение ввести строку со слешем в меню для отмены
                    cx.answer(format!("Введите пояснения для блюда"))
                    .reply_markup(cmd::Caterer::slash_markup())
                    .send()
                    .await?;

                    // Переходим в режим ввода информации о блюде
                    next(cmd::Dialogue::CatEditDishInfo(rest_id, dish_id))
                }

                // Переключение активности блюда
                cmd::CatDish::TogglePause(rest_id, dish_id) => {
                    // Запрос доп.данных не требуется, сразу переключаем активность
                    db::rest_dish_toggle(rest_id, dish_id).await;

                    // Покажем изменённую информацию
                    let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
                    next_with_info(DialogueDispatcherHandlerCx::new(bot, update, (rest_id, dish_id))).await
                }

                // Изменить группу блюда
                cmd::CatDish::EditGroup(rest_id, dish_id) => {

                    // Отправляем приглашение ввести строку с категориями в меню для выбора
                    cx.answer(format!("Введите номер группы"))
                    .reply_markup(cmd::Caterer::main_menu_markup())
                    .send()
                    .await?;

                    // Переходим в режим ввода информации о блюде
                    next(cmd::Dialogue::CatEditDishGroup(rest_id, dish_id))
                }

                // Изменить цену блюда
                cmd::CatDish::EditPrice(rest_id, dish_id) => {

                    // Отправляем приглашение ввести строку с категориями в меню для выбора
                    cx.answer(format!("Введите сумму в тыс. донгов"))
                    .reply_markup(cmd::Caterer::main_menu_markup())
                    .send()
                    .await?;

                    // Переходим в режим ввода информации о блюде
                    next(cmd::Dialogue::CatEditDishPrice(rest_id, dish_id))
                }

                // Изменить картинку
                cmd::CatDish::EditImage(rest_id, dish_id) => {

                    // Отправляем приглашение ввести строку с категориями в меню для выбора
                    cx.answer(format!("Загрузите картинку"))
                    .reply_markup(cmd::Caterer::main_menu_markup())
                    .send()
                    .await?;

                    // Переходим в режим ввода информации о блюде
                    next(cmd::Dialogue::CatEditDishImage(rest_id, dish_id))
                }

                // Удалить блюдо
                cmd::CatDish::Remove(rest_id, dish_id) => {

                    // Сначала запомним группу, в которой было блюдо - пригодится
                    let group_id = db::dish_group(rest_id, dish_id).await;
                    
                    // Удаяем
                    db::rest_dish_remove(rest_id, dish_id).await;

                    // Блюда больше нет, показываем меню группы
                    let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
                    cat_group::next_with_info(DialogueDispatcherHandlerCx::new(bot, update, (rest_id, group_id))).await
                }

                // Ошибочная команда
                cmd::CatDish::UnknownCommand => {
                    let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
                    next_with_cancel(DialogueDispatcherHandlerCx::new(bot, update, (rest_id, dish_id)), "Вы в меню блюда: неизвестная команда").await
                }
            }
        }
    }
}

// Изменение названия rest_id, dish_id
//
pub async fn edit_title_mode(cx: cmd::Cx<(i32, i32)>) -> cmd::Res {
    
    if let Some(text) = cx.update.text() {
        // Удалим из строки слеши
        let s = cmd::remove_slash(text).await;

        // Если строка не пустая, продолжим
        if !s.is_empty() {
            // Извлечём параметры
            let (rest_id, dish_id) = cx.dialogue;
        
            // Сохраним новое значение в БД
            db::rest_dish_edit_title(rest_id, dish_id, s).await;

            // Покажем изменённую информацию о группе
            return next_with_info(cx).await;
        }
    } 
    // Сообщим об отмене
    next_with_cancel(cx, "Отмена").await
}

// Изменение описания rest_id, dish_id
//
pub async fn edit_info_mode(cx: cmd::Cx<(i32, i32)>) -> cmd::Res {
    if let Some(text) = cx.update.text() {
        // Удалим из строки слеши
        let s = cmd::remove_slash(text).await;

        // Если строка не пустая, продолжим
        if !s.is_empty() {
            // Извлечём параметры
            let (rest_id, dish_id) = cx.dialogue;
        
            // Сохраним новое значение в БД
            db::rest_dish_edit_info(rest_id, dish_id, s).await;

            // Покажем изменённую информацию о группе
            return next_with_info(cx).await;
        }
    } 
    // Сообщим об отмене
    next_with_cancel(cx, "Отмена").await
}

// Изменение группы блюда rest_id, dish_id
//
pub async fn edit_dish_group_mode(cx: cmd::Cx<(i32, i32)>) -> cmd::Res {
    if let Some(text) = cx.update.text() {
        // Попытаемся преобразовать ответ пользователя в код группы
        let group_id = text.parse::<i32>().unwrap_or_default();

        // Если группа не пустая, продолжим
        if group_id > 0 {
            // Извлечём параметры
            let (rest_id, dish_id) = cx.dialogue;
        
            // Сохраним новое значение в БД
            if db::rest_dish_edit_group(rest_id, dish_id, group_id).await {
                // Покажем изменённую информацию о группе
                next_with_info(cx).await
            } else {
                // Сообщим об ошибке
                next_with_cancel(cx, "Группы с таким кодом нет, отмена").await
            }
        } else {
            // Сообщим об ошибке
            next_with_cancel(cx, "Должно быть число 1 или больше, отмена").await
        }
    } else {
        next_with_cancel(cx, "Ошибка, отмена").await
    }
}

// Изменение цены rest_id, dish_id
//
pub async fn edit_price_mode(cx: cmd::Cx<(i32, i32)>) -> cmd::Res {
    if let Some(text) = cx.update.text() {
        // Попытаемся преобразовать ответ пользователя в число
        let price = text.parse::<i32>().unwrap_or_default();

        // Извлечём параметры
        let (rest_id, dish_id) = cx.dialogue;
        
        // Сохраним новое значение в БД
        db::rest_dish_edit_price(rest_id, dish_id, price).await;
    }
    // Покажем изменённую информацию о группе
    next_with_info(cx).await
}

// Изменение картинки
//
pub async fn edit_image_mode(cx: cmd::Cx<(i32, i32)>) -> cmd::Res {
    if let Some(photo_size) = cx.update.photo() {
        // Попытаемся преобразовать ответ пользователя в число
        let image = &photo_size[0].file_id;

        // Извлечём параметры
        let (rest_id, dish_id) = cx.dialogue;
        
        // Сохраним новое значение в БД
        db::rest_dish_edit_image(rest_id, dish_id, image).await;
    }
    // Покажем изменённую информацию о группе
    next_with_info(cx).await
}
