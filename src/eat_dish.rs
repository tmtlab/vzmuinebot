/* ===============================================================================
Бот для сбора меню у рестораторов и выдача их желающим покушать.
Режим едока, выбор блюда после выбора группы и ресторана. 09 June 2020.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use teloxide::{
   prelude::*, 
   types::{InputFile, ReplyMarkup, CallbackQuery, InlineKeyboardButton, 
      ChatOrInlineMessage, InlineKeyboardMarkup, ChatId, InputMedia
   },
};
use arraylib::iter::IteratorExt;

use crate::commands as cmd;
use crate::database as db;
use crate::eater;
use crate::eat_group;
use crate::eat_group_now;
use crate::basket;
use crate::language as lang;

// Основную информацию режима
//
pub async fn next_with_info(cx: cmd::Cx<(bool, i32, i32, i32)>) -> cmd::Res {
   // Извлечём параметры
   let (compact_mode, cat_id, rest_id, group_id) = cx.dialogue;
   
   // Получаем информацию из БД
   match db::dishes_by_restaurant_and_group(rest_id, group_id).await {
      None => {
         // Такая ситуация может возникнуть, если ресторатор удалил блюда только что
         let s = String::from("Подходящие блюда исчезли");
         let new_cx = DialogueDispatcherHandlerCx::new(cx.bot, cx.update, ());
         cmd::send_text(&new_cx, &s, cmd::EaterRest::markup()).await;
      }
      Some(info) => {

         // Сформируем строку вида "название /ссылка\n"
         let s: String = if info.dishes.is_empty() {
            String::from(lang::t("ru", lang::Res::EatDishEmpty))
         } else {
            info.dishes.into_iter().map(|(key, value)| (format!("   {} /dish{}\n", value, key))).collect()
         };
         
         // Добавляем блюда к информации о группе
         let s = format!("{}\n{}", info.info, s);

         // Отображаем список блюд
         cx.answer(s)
         .reply_markup(cmd::EaterGroup::markup())
         .disable_notification(true)
         .send()
         .await?;
      }
   };

   // Переходим (остаёмся) в режим выбора ресторана
   next(cmd::Dialogue::EatRestGroupDishSelectionMode(compact_mode, cat_id, rest_id, group_id))
}

// Показывает сообщение об ошибке/отмене без повторного вывода информации
async fn next_with_cancel(cx: cmd::Cx<(bool, i32, i32, i32)>, text: &str) -> cmd::Res {
   cx.answer(text)
   .reply_markup(cmd::EaterDish::markup())
   .disable_notification(true)
   .send()
   .await?;

   // Извлечём параметры
   let (compact_mode, cat_id, rest_id, group_id) = cx.dialogue;

   // Остаёмся в прежнем режиме.
   next(cmd::Dialogue::EatRestGroupDishSelectionMode(compact_mode, cat_id, rest_id, group_id))
}



// Обработчик команд
//
pub async fn handle_selection_mode(cx: cmd::Cx<(bool, i32, i32, i32)>) -> cmd::Res {
   // Извлечём параметры
   let (compact_mode, cat_id, rest_id, group_id) = cx.dialogue;

   // Разбираем команду.
   match cx.update.text() {
      None => {
         next_with_cancel(cx, "Текстовое сообщение, пожалуйста!").await
      }
      Some(command) => {
         match cmd::EaterDish::from(command) {

            // В корзину
            cmd::EaterDish::Basket => {
               // Код едока
               let user_id = cx.update.from().unwrap().id;
               
               // Переходим в корзину
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               basket::next_with_info(DialogueDispatcherHandlerCx::new(bot, update, user_id)).await
            }

            // В главное меню
            cmd::EaterDish::Main => {
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               eater::start(DialogueDispatcherHandlerCx::new(bot, update, ()), false).await
            }

            // В предыдущее меню
            cmd::EaterDish::Return => {
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;

               // Попасть сюда могли двумя путями и это видно по коду категории
               if cat_id > 0 {
                  eat_group::next_with_info(DialogueDispatcherHandlerCx::new(bot, update, (compact_mode, cat_id, rest_id))).await
               } else {
                  eat_group_now::next_with_info(DialogueDispatcherHandlerCx::new(bot, update, (compact_mode, rest_id))).await
               }
            }

            // Выбор блюда
            cmd::EaterDish::Dish(dish_num) => {
               // Получаем информацию из БД
               let (info, dish_image_id) = match db::eater_dish_info(rest_id, group_id, dish_num).await {
                  Some(dish_info) => dish_info,
                  None => (format!("Ошибка db::eater_dish_info({}, {}, {})", rest_id, group_id, dish_num), None)
               };

               // Идентифицируем пользователя
               let user_id = cx.update.from().unwrap().id;

               // Запросим из БД, сколько этих блюд пользователь уже выбрал
               let ordered_amount = db::amount_in_basket(rest_id, group_id, dish_num, user_id).await;

               // Создаём инлайн кнопки с указанием количества блюд
               let inline_keyboard = cmd::EaterDish::inline_markup(&db::make_key_3_int(rest_id, group_id, dish_num), ordered_amount);

               // Отображаем информацию о блюде и оставляем кнопки главного меню. Если для блюда задана картинка, то текст будет комментарием
               if let Some(image_id) = dish_image_id {
                  // Создадим графический объект
                  let image = InputFile::file_id(image_id);

                  // Отправляем картинку и текст как комментарий
                  cx.answer_photo(image)
                  .caption(info)
                  .reply_markup(ReplyMarkup::InlineKeyboardMarkup(inline_keyboard))
                  .disable_notification(true)
                  .send()
                  .await?;
                  
                  next(cmd::Dialogue::EatRestGroupDishSelectionMode(compact_mode, cat_id, rest_id, group_id))
               } else {
                  cx.answer(info)
                  .reply_markup(inline_keyboard)
                  .disable_notification(true)
                  .send()
                  .await?;

                  next(cmd::Dialogue::EatRestGroupDishSelectionMode(compact_mode, cat_id, rest_id, group_id))
               }
            }

            cmd::EaterDish::UnknownCommand => {
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               next_with_cancel(DialogueDispatcherHandlerCx::new(bot, update, (compact_mode, cat_id, rest_id, group_id)), "Вы в меню выбора блюда: неизвестная команда").await
            }
         }
      }
   }
}

// Выводит инлайн кнопки
//
pub async fn show_inline_interface(cx: &DispatcherHandlerCx<CallbackQuery>, rest_num: i32, group_num: i32, cat_id: i32) -> bool {
   // db::log(&format!("eat_dish::show_inline_interface ({}_{}_{})", rest_num, group_num, cat_id)).await;

   // Если категория не задана, запросим её из базы
   let cat_id = if cat_id == 0 { db::category_by_restaurant_and_group(rest_num, group_num).await } else { cat_id };

   // Получаем информацию из БД - нужен текст, картинка и кнопки
   let (text, markup) = match db::dishes_by_restaurant_and_group(rest_num, group_num).await {
      None => {
         // Такая ситуация может возникнуть, если ресторатор удалил группу только что

         // Кнопка назад
         let buttons = vec![InlineKeyboardButton::callback(String::from("Назад"), format!("rrg{}", db::make_key_3_int(rest_num, cat_id, 0)))];
         // Формируем меню
         let markup = InlineKeyboardMarkup::default()
         .append_row(buttons);

         // Сформированные данные
         (String::from("Подходящие блюда исчезли"), markup)
      }
      Some(info) => {
         // Создадим кнопки
         let buttons: Vec<InlineKeyboardButton> = info.dishes.into_iter()
         .map(|(key, value)| (InlineKeyboardButton::callback(value, format!("dis{}", db::make_key_3_int(rest_num, group_num, key)))))
         .collect();

         let (long, mut short) : (Vec<_>, Vec<_>) = buttons
         .into_iter()
         .partition(|n| n.text.chars().count() > 21);
      
         // Последняя непарная кнопка, если есть
         let last = if short.len() % 2 == 1 { short.pop() } else { None };
      
         // Сначала длинные кнопки по одной
         let markup = long.into_iter() 
         .fold(InlineKeyboardMarkup::default(), |acc, item| acc.append_row(vec![item]));
      
         // Короткие по две в ряд
         let markup = short.into_iter().array_chunks::<[_; 2]>()
         .fold(markup, |acc, [left, right]| acc.append_row(vec![left, right]));
      
         // Кнопка назад
         let button_back = InlineKeyboardButton::callback(String::from("Назад"), format!("rrg{}", db::make_key_3_int(rest_num, cat_id, 0)));

         // Добавляем последнюю непарную кнопку и кнопку назад
         let markup = if let Some(last_button) = last {
            markup.append_row(vec![last_button, button_back])
         } else {
            markup.append_row(vec![button_back])
         };

         // Сформированные данные
         (info.info, markup)
      }
   };

   // Достаём chat_id
   let message = cx.update.message.as_ref().unwrap();
   let chat_message = ChatOrInlineMessage::Chat {
      chat_id: ChatId::Id(message.chat_id()),
      message_id: message.id,
   };

   // Приготовим структуру для редактирования
   let media = InputMedia::Photo{
      media: InputFile::file_id(db::default_photo_id()),
      caption: Some(text),
      parse_mode: None,
   };

   // Отправляем изменения
   match cx.bot.edit_message_media(chat_message, media)
   .reply_markup(markup)
   .send()
   .await {
      Err(e) => {
         db::log(&format!("Error eat_dish::show_inline_interface {}", e)).await;
         false
      }
      _ => true,
   }
}

// Показывает информацию о блюде
//
pub async fn show_dish(cx: &DispatcherHandlerCx<CallbackQuery>, rest_num: i32, group_num: i32, dish_num: i32) -> bool {
   // Получаем информацию из БД
   let (info, dish_image_id) = match db::eater_dish_info(rest_num, group_num, dish_num).await {
      Some(dish_info) => dish_info,
      None => {
         db::log(&format!("Error eat_dish::show_dish_info({}, {}, {})", rest_num, group_num, dish_num)).await;
         return false;
      }
   };

   // Идентифицируем пользователя
   let user_id = cx.update.from.id;

   // Запросим из БД, сколько этих блюд пользователь уже выбрал
   let ordered_amount = db::amount_in_basket(rest_num, group_num, dish_num, user_id).await;

   // Создаём инлайн кнопки с указанием количества блюд
   let button_back = InlineKeyboardButton::callback(String::from("Назад"), format!("rrd{}", db::make_key_3_int(rest_num, group_num, 0)));
   let inline_keyboard = cmd::EaterDish::inline_markup(&db::make_key_3_int(rest_num, group_num, dish_num), ordered_amount)
   .append_to_row(button_back, 0);
   // db::log(&format!("rrg{}", db::make_key_3_int(est_num, group_num, cat_id))).await;



   // Картинка блюда
   let photo_id = match dish_image_id {
      Some(photo) => photo,
      None => db::default_photo_id(),
   };

   // Создадим графический объект
   let image = InputFile::file_id(photo_id);

   // Достаём chat_id
   let message = cx.update.message.as_ref().unwrap();
   let chat_id = ChatId::Id(message.chat_id());

   // Отображаем информацию о блюде
   match cx.bot.send_photo(chat_id, image)
   .caption(info)
   .reply_markup(ReplyMarkup::InlineKeyboardMarkup(inline_keyboard))
   .disable_notification(true)
   .send()
   .await {
      Err(e) => {
         db::log(&format!("Error eat_dish::show_inline_interface {}", e)).await;
         false
      }
      _ => true,
   }
}