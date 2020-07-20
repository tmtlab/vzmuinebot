/* ===============================================================================
Бот для сбора меню у рестораторов и выдача их желающим покушать.
Начало диалога и обработка в режиме едока. 01 June 2020.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use teloxide::{
    prelude::*,
};

use crate::commands as cmd;
use crate::database as db;
use crate::eat_rest;
use crate::eat_rest_now;
use crate::basket;
use crate::settings;
use crate::gear;
use crate::eat_dish;

pub async fn start(cx: cmd::Cx<()>, after_restart: bool) -> cmd::Res {
   
   // Различаем перезапуск и возврат из меню ресторатора
   let s = if after_restart {
      // Это первый вход пользователя после перезапуска, сообщим об этом
      let text = format!("{} начал сеанс", db::user_info(cx.update.from(), true));
      settings::log(&text).await;

      // Для администратора отдельное приветствие
      if settings::is_admin(cx.update.from()) {
         String::from("Начат новый сеанс. Список команд администратора в описании: https://github.com/ArtHome12/vzmuinebot")
      } else {
         String::from("Начат новый сеанс. Пожалуйста, выберите в основном меню снизу какие заведения показать.")
      }
   } else {
      String::from("Пожалуйста, выберите в основном меню снизу какие заведения показать.")
   };
   
   // Запросим настройку пользователя с режимом интерфейса и обновим время последнего входа в БД
   let now = settings::current_date_time();
   let compact_mode = db::user_compact_interface(cx.update.from(), now).await;

   // Если сессия началась с какой-то команды, то попробуем сразу её обработать
   if let Some(input) = cx.update.text() {
      // Пытаемся распознать команду как собственную или глобальную
      let known = cmd::User::from(input) != cmd::User::UnknownCommand || cmd::Common::from(input) != cmd::Common::UnknownCommand;
      if known {
         let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
         return handle_commands(DialogueDispatcherHandlerCx::new(bot, update, compact_mode)).await;
      }
   }

   // Если команды не было или она не распознана, отображаем приветственное сообщение и меню с кнопками.
   cmd::send_text(&DialogueDispatcherHandlerCx::new(cx.bot, cx.update.clone(), ()), &s, cmd::User::main_menu_markup()).await;
   
   // Код едока
/*   let user_id = cx.update.from().unwrap().id;

   
   if let Some(input) = cx.update.text() {
   }*/

   // Переходим в режим получения выбранного пункта в главном меню
   next(cmd::Dialogue::UserMode(compact_mode))
}

pub async fn handle_commands(cx: cmd::Cx<bool>) -> cmd::Res {
   // Режим интерфейса
   let compact_mode = cx.dialogue;

   // Разбираем команду
   match cx.update.text() {
      None => {
         let s = match cx.update.photo() {
            Some(photo_size) => format!("Вы прислали картинку с id\n{}", &photo_size[0].file_id),
            None => String::from("Текстовое сообщение, пожалуйста!"),
         };
         cmd::send_text(&DialogueDispatcherHandlerCx::new(cx.bot, cx.update, ()), &s, cmd::User::main_menu_markup()).await
      }
      Some(command) => {
         match cmd::User::from(command) {
            cmd::User::Category(cat_id) => {
               // Отобразим все рестораны, у которых есть в меню выбранная категория и переходим в режим выбора ресторана
               return eat_rest::next_with_info(DialogueDispatcherHandlerCx::new(cx.bot, cx.update, (compact_mode, cat_id))).await;
            }
            cmd::User::OpenedNow => {
               // Отобразим рестораны, открытые сейчас и перейдём в режим их выбора
               return eat_rest_now::next_with_info(cx).await;
            }
            cmd::User::UnknownCommand => {
               // Сохраним текущее состояние для возврата
               let origin = Box::new(cmd::DialogueState{ d : cmd::Dialogue::UserMode(compact_mode), m : cmd::User::main_menu_markup()});

               // Возможно это общая команда
               if let Some(res) = handle_common_commands(DialogueDispatcherHandlerCx::new(cx.bot.clone(), cx.update.clone(), ()), command, origin).await {return res;}
               else {
                  let s = &format!("Вы в главном меню: неизвестная команда {}", command);
                  cmd::send_text(&DialogueDispatcherHandlerCx::new(cx.bot, cx.update, ()), s, cmd::User::main_menu_markup()).await
               }
            }
            cmd::User::Gear => {
               // Переходим в меню с шестерёнкой
               return gear::next_with_info(DialogueDispatcherHandlerCx::new(cx.bot, cx.update, compact_mode)).await;
            }
            cmd::User::Basket => {
               // Код едока
               let user_id = cx.update.from().unwrap().id;
               
               // Переходим в корзину
               return basket::next_with_info(DialogueDispatcherHandlerCx::new(cx.bot, cx.update, user_id)).await;
            }
            cmd::User::ChatId => {
               // Отправим информацию о чате
               let id = cx.chat_id();
               cmd::send_text(&DialogueDispatcherHandlerCx::new(cx.bot, cx.update, ()), &format!("Chat id={}", id), cmd::User::main_menu_markup()).await;
            }
         }
      }
   }

   // Остаёмся в пользовательском режиме.
   next(cmd::Dialogue::UserMode(compact_mode))
}

// Обработка глобальных команд
pub async fn handle_common_commands(cx: cmd::Cx<()>, command: &str, origin : Box<cmd::DialogueState>) -> Option<cmd::Res> {

   match cmd::Common::from(command) {
      cmd::Common::Start => {
         // Отображаем приветственное сообщение и меню с кнопками
         let s = "Пожалуйста, выберите в основном меню снизу какие заведения показать.";
         cmd::send_text(&DialogueDispatcherHandlerCx::new(cx.bot, cx.update.clone(), ()), s, cmd::User::main_menu_markup()).await;

         // Запросим настройку пользователя с режимом интерфейса и обновим время последнего входа в БД
         let now = settings::current_date_time();
         let compact_mode = db::user_compact_interface(cx.update.from(), now).await;

         Some(next(cmd::Dialogue::UserMode(compact_mode)))
      }
      cmd::Common::StartArgs(first, second, third) => {
         // Запросим настройку пользователя с режимом интерфейса и обновим время последнего входа в БД
         let now = settings::current_date_time();
         let compact_mode = db::user_compact_interface(cx.update.from(), now).await;

         // Если третий аргумент нулевой, надо отобразить группу
         if third == 0 {
            let new_cx = DialogueDispatcherHandlerCx::new(cx.bot, cx.update, (compact_mode, 0, first, second));
            Some(eat_dish::next_with_info(new_cx).await)
         } else {
            let new_cx = DialogueDispatcherHandlerCx::new(cx.bot, cx.update, (compact_mode, 0, first, second));
            Some(eat_dish::next_with_info(new_cx).await)
         }
      }
      cmd::Common::SendMessage(caterer_id) => {
         // Отправляем приглашение ввести строку со слешем в меню для отмены
         let res = cx.answer(format!("Введите сообщение (/ для отмены)"))
         .reply_markup(cmd::Caterer::slash_markup())
         .disable_notification(true)
         .send()
         .await;

         if let Ok(_) = res {
            // Код едока
            let user_id = cx.update.from().unwrap().id;

            // Переходим в режим ввода
            Some(next(cmd::Dialogue::MessageToCaterer(user_id, caterer_id, origin)))
         } else {None}
      }
      cmd::Common::UnknownCommand => None,
   }
}
