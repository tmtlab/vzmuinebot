/* ===============================================================================
Бот для сбора меню у рестораторов и выдача их желающим покушать.
Команды бота и меню. 31 May 2020.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use teloxide::{
   prelude::*, 
   types::{KeyboardButton, ReplyKeyboardMarkup, InlineKeyboardMarkup, 
      InlineKeyboardButton, ReplyMarkup, InputFile, ButtonRequest, 
   },
};

use crate::database as db;
use crate::settings;

// ============================================================================
// [Common]
// ============================================================================
#[derive(SmartDefault)]
pub enum Dialogue {
   #[default]
   Start,
   UserMode,
   UserModeEditCatImage(String), // new image id
   EatRestSelectionMode(i32), // cat_id
   EatRestGroupSelectionMode(i32, i32), // cat_id, rest_id
   EatRestGroupDishSelectionMode(i32, i32, i32), // cat_id, rest_id, group_id
   EatRestNowSelectionMode,
   EatRestGroupNowSelectionMode(i32), // rest_id
   CatererMode(i32), // rest_id
   CatEditRestTitle(i32), // rest_id
   CatEditRestInfo(i32), // rest_id
   CatEditRestImage(i32), // rest_id
   CatEditGroup(i32, i32), // rest_id, group_id
   CatAddGroup(i32), // rest_id
   CatEditGroupTitle(i32, i32), // rest_id, group_id (cat_group)
   CatEditGroupInfo(i32, i32), // rest_id, group_id (cat_group)
   CatEditGroupCategory(i32, i32), // rest_id, group_id (cat_group)
   CatEditGroupTime(i32, i32), // rest_id, group_id (cat_group)
   CatAddDish(i32, i32), // rest_id, dish_id (cat_group)
   CatEditDish(i32, i32, i32), // rest_num, group_num, dish_num (dish)
   CatEditDishTitle(i32, i32, i32), // rest_num, group_num, dish_num (dish)), // rest_id, dish_id (dish)
   CatEditDishInfo(i32, i32, i32), // rest_num, group_num, dish_num (dish)), // rest_id, dish_id (dish)
   CatEditDishGroup(i32, i32, i32), // rest_num, group_num, dish_num (dish)), // rest_id, dish_id (dish)
   CatEditDishPrice(i32, i32, i32), // rest_num, group_num, dish_num (dish)), // rest_id, dish_id (dish)
   CatEditDishImage(i32, i32, i32), // rest_num, group_num, dish_num (dish)), // rest_id, dish_id (dish)
   BasketMode(i32), // user_id
   BasketEditName(i32), // user_id
   BasketEditContact(i32), // user_id
   BasketEditAddress(i32), // user_id
   MessageToCaterer(i32, i32, Box<DialogueState>), // user_id, caterer_id, previous mode
   GearMode,
}

pub type Cx<State> = DialogueDispatcherHandlerCx<Message, State>;
pub type Res = ResponseResult<DialogueStage<Dialogue>>;

// Структура для сохранения состояния диалога вместе с меню
pub struct DialogueState {
   pub d  : Dialogue,
   pub m  : ReplyKeyboardMarkup,
}

// ============================================================================
// [Common commands]
// ============================================================================
#[derive(Copy, Clone, PartialEq)]
pub enum Common {
   Start,
   StartArgs(i32, i32, i32), // rest_num, group_num, dish_num
   SendMessage(i32), // caterer_id
   Goto(i32, i32, i32),   // rest_num, group_num, dish_num
   UnknownCommand,
}

impl Common {
   pub fn from(input: &str) -> Common {
      match input {
         "/start" => Common::Start,
         _ => {
            // Ищем среди команд с аргументами
            if input.get(..4).unwrap_or_default() == "/snd" {
               let r_part = input.get(4..).unwrap_or_default();
               Common::SendMessage(r_part.parse().unwrap_or_default())
            } else if input.get(..7).unwrap_or_default() == "/start " {
               let r_part = input.get(7..).unwrap_or_default();
               if let Ok((first, second, third)) = db::parse_key_3_int(r_part) {Common::StartArgs(first, second, third)}
               else {Common::UnknownCommand}
            } else if input.get(..5).unwrap_or_default() == "/goto" {
               let r_part = input.get(5..).unwrap_or_default();
               if let Ok((first, second, third)) = db::parse_key_3_int(r_part) {Common::Goto(first, second, third)}
               else {Common::UnknownCommand}
            } else {Common::UnknownCommand}
         }
      }
   }
}


// ============================================================================
// [Client menu]
// ============================================================================
#[derive(Copy, Clone, PartialEq)]
pub enum User {
    // Команды главного меню
    Category(i32),   // cat_id 
    OpenedNow,
    Basket,
    UnknownCommand,
    ChatId,
    Gear,
}

impl User {
   pub fn from(input: &str) -> User {
      match input {
         // Сначала проверим на цельные команды.
         "Соки воды" => User::Category(1),
         "Еда" => User::Category(2),
         "Напитки" => User::Category(3),
         "Развлечения" => User::Category(4),
         "Сейчас" => User::OpenedNow,
         "🛒Корзина" => User::Basket,
         "⚙" => User::Gear,
         _ => {
            // Ищем среди команд с цифровыми суффиксами - аргументами
            match input.get(..5).unwrap_or_default() {
               "/chat" => User::ChatId, // правее может быть имя бота, игнорируем это.
               _ => User::UnknownCommand,
            }
         }
      }
   }

    pub fn main_menu_markup() -> ReplyKeyboardMarkup {
      ReplyKeyboardMarkup::default()
         .append_row(vec![
            // KeyboardButton::new("Соки воды"),
            KeyboardButton::new("Еда"),
            KeyboardButton::new("Напитки"),
            KeyboardButton::new("Развлечения"),
         ])
         .append_row(vec![
            KeyboardButton::new("🛒Корзина"),
            KeyboardButton::new("Сейчас"),
            KeyboardButton::new("⚙"),
         ])
         .resize_keyboard(true)
   }
}

// ============================================================================
// [Restaurant owner main menu]
// ============================================================================
#[derive(Copy, Clone)]
pub enum Caterer {
   // Команды главного меню
   Main(i32), // rest_id
   Exit, 
   UnknownCommand,
   // Добавляет нового ресторатора user_id или возобновляет его доступ.
   //Registration(u32),
   // Приостанавливает доступ ресторатора user_id и скрывает его меню.
   //Hold(u32),
   // Изменить название ресторана
   EditTitle(i32), // rest_id
   // Изменить описание ресторана
   EditInfo(i32), // rest_id
   // Доступность меню, определяемая самим пользователем
   TogglePause(i32), // rest_id
   // Фото престорана
   EditImage(i32), // rest_id
   // Переход к редактированию указанной группы блюд.
   EditGroup(i32, i32), // rest_id, group_id
   // Добавляет новую группу
   AddGroup(i32), // rest_id
   // Передача владения новому ресторатору
   TransferOwnership(i32, i32), // rest_id, user_id
   // Рекламировать
   Promote(i32), // rest_id
}

impl Caterer {

   pub fn from(rest_id: i32, input: &str) -> Caterer {
      match input {
         // Сначала проверим на цельные команды.
         "Главная" => Caterer::Main(rest_id),
         "Выход" => Caterer::Exit,
         "/EditTitle" => Caterer::EditTitle(rest_id),
         "/EditInfo" => Caterer::EditInfo(rest_id),
         "/Toggle" => Caterer::TogglePause(rest_id),
         "/EditImg" => Caterer::EditImage(rest_id),
         "/AddGroup" => Caterer::AddGroup(rest_id),
         "/Promote" => Caterer::Promote(rest_id),
         _ => {
               // Ищем среди команд с цифровыми суффиксами - аргументами
               match input.get(..5).unwrap_or_default() {
                  "/EdGr" => Caterer::EditGroup(rest_id, input.get(5..).unwrap_or_default().parse().unwrap_or_default()),
                  "/move" => Caterer::TransferOwnership(rest_id, input.get(5..).unwrap_or_default().parse().unwrap_or_default()),
                  _ => Caterer::UnknownCommand,
               }
         }
      }
   }

   pub fn main_menu_markup() -> ReplyKeyboardMarkup {
      ReplyKeyboardMarkup::default()
         .append_row(vec![
               KeyboardButton::new("Главная"),
               KeyboardButton::new("Выход"),
         ])
         .resize_keyboard(true)
         //.one_time_keyboard(true)
   }

   pub fn slash_markup() -> ReplyKeyboardMarkup {
      ReplyKeyboardMarkup::default()
         .append_row(vec![
               KeyboardButton::new("/"),
         ])
         .resize_keyboard(true)
   }
}

// ============================================================================
// [Some]
// ============================================================================
pub async fn remove_slash(s: &str) -> String {
   // Если строка начинается c косой черты, значит это не данные, а команда
   if s.get(..1).unwrap_or_default() == "/" {
      String::default()
   } else {
      String::from(s)  //.replace("/", "") делает невозможным ввод веб-ссылок
   }
}

// Отправляет текстовое сообщение
pub async fn send_text<T>(cx: &Cx<()>, text: &str, markup: T) 
where
   T: Into<ReplyMarkup>,
{
   let res = cx.answer(text)
   .reply_markup(markup)
   .disable_notification(true)
   .disable_web_page_preview(true)
   .send()
   .await;

   // Если не удалось отправить, выведем ошибку в лог
   if let Err(err) = res {
      settings::log(&format!("Error send_text({}): {}", text, err)).await;
   }
}

// Отправляет текстовое сообщение
pub async fn send_text_without_markup(cx: &Cx<()>, text: &str) 
{
   let res = cx.answer(text)
   .disable_notification(true)
   .disable_web_page_preview(true)
   .send()
   .await;

   // Если не удалось отправить, выведем ошибку в лог
   if let Err(err) = res {
      settings::log(&format!("Error send_text_without_markup({}): {}", text, err)).await;
   }
}

// Отправляет картинку
//
pub async fn send_photo(cx: &Cx<()>, text: &str, markup: ReplyMarkup, image_id : String) 
{
   // Отправляем картинку и текст как комментарий
   let res = cx.answer_photo(InputFile::file_id(image_id))
      .caption(text)
      .reply_markup(markup)
      .disable_notification(true)
      .send()
      .await;

   // Если не удалось отправить, выведем ошибку в лог
   if let Err(err) = res {
      settings::log(&format!("Error send_photo({}): {}", text, err)).await;
   }
}


// ============================================================================
// [Restaurant group editing menu]
// ============================================================================
#[derive(Copy, Clone)]
pub enum CatGroup {
    // Команды главного меню
    Main(i32), // rest_id
    Exit, 
    UnknownCommand,
    // Изменить название группы
    EditTitle(i32, i32), // rest_id, group_id
    // Изменить описание группы
    EditInfo(i32, i32), // rest_id, group_id
    // Переключить доступность группы
    TogglePause(i32, i32), // rest_id, group_id
    // Изменить категорию группы
    EditCategory(i32, i32), // rest_id, group_id
    // Изменить время доступности группы
    EditTime(i32, i32), // rest_id, group_id
    // Удалить группу
    RemoveGroup(i32, i32), // rest_id, group_id
    // Добавление нового блюда
    AddDish(i32, i32), // rest_id, group_id
    // Редактирование блюда
    EditDish(i32, i32, i32), // rest_id, group_id, dish_id
    // Рекламировать
    Promote(i32, i32), // rest_id, group_id
}

impl CatGroup {

    pub fn from(rest_id: i32, group_id: i32, input: &str) -> CatGroup {
        match input {
            // Сначала проверим на цельные команды.
            "Главная" => CatGroup::Main(rest_id),
            "Выход" => CatGroup::Exit,
            "/EditTitle" => CatGroup::EditTitle(rest_id, group_id),
            "/EditInfo" => CatGroup::EditInfo(rest_id, group_id),
            "/Toggle" => CatGroup::TogglePause(rest_id, group_id),
            "/EditCat" => CatGroup::EditCategory(rest_id, group_id),
            "/EditTime" => CatGroup::EditTime(rest_id, group_id),
            "/Remove" => CatGroup::RemoveGroup(rest_id, group_id),
            "/AddDish" => CatGroup::AddDish(rest_id, group_id),
            "/Promote" => CatGroup::Promote(rest_id, group_id),
            _ => {
                // Ищем среди команд с цифровыми суффиксами - аргументами
                match input.get(..5).unwrap_or_default() {
                    "/EdDi" => CatGroup::EditDish(rest_id, group_id, input.get(5..).unwrap_or_default().parse().unwrap_or_default()),
                    _ => CatGroup::UnknownCommand,
                }
            }
        }
    }

    pub fn category_markup() -> ReplyKeyboardMarkup {
        ReplyKeyboardMarkup::default()
            .append_row(vec![
               //  KeyboardButton::new("Соки воды"),
                KeyboardButton::new("Еда"),
                KeyboardButton::new("Напитки"),
                KeyboardButton::new("Развлечения"),
            ])
            .resize_keyboard(true)
    }
}


// ============================================================================
// [Restaurant dish editing menu]
// ============================================================================
#[derive(Copy, Clone)]
pub enum CatDish {
    // Команды главного меню
    Main(i32), // rest_id
    Exit, 
    UnknownCommand,
    // Изменить название
    EditTitle(i32, i32, i32), // rest_id, group_id, dish_id
    // Изменить описание
    EditInfo(i32, i32, i32), // rest_id, group_id, dish_id
    // Переключить доступность
    TogglePause(i32, i32, i32), // rest_id, group_id, dish_id
    // Изменить группу
    EditGroup(i32, i32, i32), // rest_id, group_id, dish_id
    // Изменить цену
    EditPrice(i32, i32, i32), // rest_id, group_id, dish_id
    // Изменить цену
    EditImage(i32, i32, i32), // rest_id, group_id, dish_id
    // Удалить
    Remove(i32, i32, i32), // rest_id, group_id, dish_id
    // Рекламировать
    Promote(i32, i32, i32), // rest_id, group_id, dish_id
}

impl CatDish {

    pub fn from(rest_id: i32, group_id: i32, dish_id: i32, input: &str) -> CatDish {
        match input {
            // Сначала проверим на цельные команды.
            "Главная" => CatDish::Main(rest_id),
            "Выход" => CatDish::Exit,
            "/EditTitle" => CatDish::EditTitle(rest_id, group_id, dish_id),
            "/EditInfo" => CatDish::EditInfo(rest_id, group_id, dish_id),
            "/Toggle" => CatDish::TogglePause(rest_id, group_id, dish_id),
            "/EditGroup" => CatDish::EditGroup(rest_id, group_id, dish_id),
            "/EditPrice" => CatDish::EditPrice(rest_id, group_id, dish_id),
            "/EditImg" => CatDish::EditImage(rest_id, group_id, dish_id),
            "/Remove" => CatDish::Remove(rest_id, group_id, dish_id),
            "/Promote" => CatDish::Promote(rest_id, group_id, dish_id),
            _ => CatDish::UnknownCommand,
        }
    }
}


// ============================================================================
// [Eater menu, restaurant selection]
// ============================================================================
#[derive(Copy, Clone)]
pub enum EaterRest {
   Basket,
   Main,
   UnknownCommand,
   Restaurant(i32),   // cat_id 
}

impl EaterRest {
   pub fn from(input: &str) -> EaterRest {
      match input {
         // Сначала проверим на цельные команды.
         "🛒" => EaterRest::Basket,
         "В начало" => EaterRest::Main,
         _ => {
             // Ищем среди команд с цифровыми суффиксами - аргументами
             match input.get(..5).unwrap_or_default() {
                 "/rest" => EaterRest::Restaurant(input.get(5..).unwrap_or_default().parse().unwrap_or_default()),
                 _ => EaterRest::UnknownCommand,
             }
         }
     }
   }

   pub fn markup() -> ReplyKeyboardMarkup {
      ReplyKeyboardMarkup::default()
          .append_row(vec![
            KeyboardButton::new("🛒"),
            KeyboardButton::new("В начало"),
          ])
          .resize_keyboard(true)
  }
}

// ============================================================================
// [Eater menu, group selection]
// ============================================================================
#[derive(Copy, Clone)]
pub enum EaterGroup {
   Basket,
   Main,
   Return,
   UnknownCommand,
   Group(i32),   // cat_id 
}

impl EaterGroup {
   pub fn from(input: &str) -> EaterGroup {
      match input {
         // Сначала проверим на цельные команды.
         "🛒" => EaterGroup::Basket,
         "В начало" => EaterGroup::Main,
         "⏪⏪Назад" => EaterGroup::Return,
         _ => {
             // Ищем среди команд с цифровыми суффиксами - аргументами
             match input.get(..5).unwrap_or_default() {
                 "/grou" => EaterGroup::Group(input.get(5..).unwrap_or_default().parse().unwrap_or_default()),
                 _ => EaterGroup::UnknownCommand,
             }
         }
     }
   }

   pub fn markup() -> ReplyKeyboardMarkup {
      ReplyKeyboardMarkup::default()
         .append_row(vec![
            KeyboardButton::new("🛒"),
            KeyboardButton::new("В начало"),
            KeyboardButton::new("⏪⏪Назад"),
         ])
         .resize_keyboard(true)
  }
}

// ============================================================================
// [Eater menu, dish selection]
// ============================================================================
#[derive(Copy, Clone)]
pub enum EaterDish {
   Basket,
   Main,
   Return,
   UnknownCommand,
   Dish(i32),   // group_id
}

impl EaterDish {
   pub fn from(input: &str) -> EaterDish {
      match input {
         // Сначала проверим на цельные команды.
         "🛒" => EaterDish::Basket,
         "В начало" => EaterDish::Main,
         "⏪Назад" => EaterDish::Return,
         _ => {
             // Ищем среди команд с цифровыми суффиксами - аргументами
             match input.get(..5).unwrap_or_default() {
                 "/dish" => EaterDish::Dish(input.get(5..).unwrap_or_default().parse().unwrap_or_default()),
                 _ => EaterDish::UnknownCommand,
             }
         }
     }
   }

   pub fn markup() -> ReplyKeyboardMarkup {
      ReplyKeyboardMarkup::default()
      .append_row(vec![
         KeyboardButton::new("🛒"),
         KeyboardButton::new("В начало"),
         KeyboardButton::new("⏪Назад"),
      ])
      .resize_keyboard(true)
   }

   pub fn inline_markup(key: &str, amount: i32) -> InlineKeyboardMarkup {
      // Если количество не пустое, добавим кнопку для убавления
      let buttons = if amount == 0 {
            vec![
               InlineKeyboardButton::callback(format!("+1 ({})", amount), format!("add{}", key)),
            ]
      } else {
         vec![
            InlineKeyboardButton::callback(format!("+1 ({})", amount), format!("add{}", key)),
            InlineKeyboardButton::callback("-1".to_string(), format!("del{}", key)),
         ]
      };

      // Формируем меню
      InlineKeyboardMarkup::default()
      .append_row(buttons)
   }
}

// ============================================================================
// [Basket menu]
// ============================================================================
#[derive(Copy, Clone)]
pub enum Basket {
   Main,
   Refresh,
   Clear,
   Delete(i32, i32, i32),  // rest_num, group_num, dish_num
   UnknownCommand,
   EditName,
   EditContact,
   EditAddress,
   TogglePickup,
}

impl Basket {
   pub fn from(input: &str) -> Basket {
      match input {
         "В начало" => Basket::Main,
         "⟳ Обновить" => Basket::Refresh,
         "Очистить" => Basket::Clear,
         "/edit_name" => Basket::EditName,
         "/edit_contact" => Basket::EditContact,
         "/edit_address" => Basket::EditAddress,
         "/toggle" => Basket::TogglePickup,
         _ => {
            // Ищем среди команд с аргументами
            let r_part = input.get(4..).unwrap_or_default();
            match input.get(..4).unwrap_or_default() {
               "/del" => {
                  // Попытаемся извлечь аргументы
                  match db::parse_key_3_int(r_part) {
                     Ok((rest_num, group_num, dish_num)) => Basket::Delete(rest_num, group_num, dish_num),
                     _ => Basket::UnknownCommand,
                  }
               }
               _ => Basket::UnknownCommand,
            }
         }
      }
   }

   // Кнопки для меню снизу
   pub fn bottom_markup() -> ReplyKeyboardMarkup {
      ReplyKeyboardMarkup::default()
      .append_row(vec![
         KeyboardButton::new("В начало"),
         KeyboardButton::new("⟳ Обновить"),
         KeyboardButton::new("Очистить"),
      ])
      .resize_keyboard(true)
   }

   pub fn address_markup() -> ReplyKeyboardMarkup {
      ReplyKeyboardMarkup::default()
         .append_row(vec![
            KeyboardButton::new("Геопозиция")
            .request(ButtonRequest::Location),
            KeyboardButton::new("/"),
         ])
         .resize_keyboard(true)
   }

   // Меню при отправке нового заказа из корзины
   pub fn inline_markup_send(rest_id: i32) -> InlineKeyboardMarkup {
      // Колбек команда
      let data = format!("bas{}", db::make_key_3_int(rest_id, 0, 0));

      let button = InlineKeyboardButton::callback(String::from("Оформить через бота"), data);

      InlineKeyboardMarkup::default()
      .append_row(vec![button])
   }

   // Меню едока для заказов в обработке
   pub fn inline_markup_message_cancel(ticket_id: i32) -> InlineKeyboardMarkup {
      // Аргументы для колбек команды
      let args = db::make_key_3_int(ticket_id, 0, 0);
      // let button1 = InlineKeyboardButton::callback(String::from("Написать"), format!("bse{}", args));
      let button2 = InlineKeyboardButton::callback(String::from("Отмена заказа"), format!("bca{}", args));

      InlineKeyboardMarkup::default()
      .append_row(vec![button2])
   }

   // Меню едока для заказов на последней стадии
   pub fn inline_markup_message_confirm(ticket_id: i32) -> InlineKeyboardMarkup {
      // Аргументы для колбек команды
      let args = db::make_key_3_int(ticket_id, 0, 0);
      let button1 = InlineKeyboardButton::callback(String::from("Отмена заказа"), format!("bca{}", args));
      let button2 = InlineKeyboardButton::callback(String::from("Подтвердить"), format!("bne{}", args));

      InlineKeyboardMarkup::default()
      .append_row(vec![button1, button2])
   }

   // Меню ресторатора для заказов в обработке
   pub fn inline_markup_message_next(ticket_id: i32) -> InlineKeyboardMarkup {
      // Аргументы для колбек команды
      let args = db::make_key_3_int(ticket_id, 0, 0);
      let button1 = InlineKeyboardButton::callback(String::from("Отмена заказа"), format!("bca{}", args));
      let button2 = InlineKeyboardButton::callback(String::from("Далее"), format!("bne{}", args));

      InlineKeyboardMarkup::default()
      .append_row(vec![button1, button2])
   }

   
}

// ============================================================================
// [Gear menu]
// ============================================================================
#[derive(Copy, Clone)]
pub enum Gear {
   Main,
   UnknownCommand,
   CatererMode,
   ToggleInterface,
   RegisterCaterer(i32), // user_id
   HoldCaterer(i32), // user_id
   Sudo(i32), // rest_num
   List,
}

impl Gear {
   pub fn from(input: &str) -> Gear {
      match input {
         "В начало" => Gear::Main,
         "Добавить меню" => Gear::CatererMode,
         "/toggle" => Gear::ToggleInterface,
         "/list" => Gear::List,
         _ => {
            // Ищем среди команд с цифровыми суффиксами - аргументами
            match input.get(..5).unwrap_or_default() {
               "/regi" => Gear::RegisterCaterer(input.get(5..).unwrap_or_default().parse().unwrap_or_default()),
               "/hold" => Gear::HoldCaterer(input.get(5..).unwrap_or_default().parse().unwrap_or_default()),
               "/sudo" => Gear::Sudo(input.get(5..).unwrap_or_default().parse().unwrap_or_default()),
               _ => Gear::UnknownCommand,
            }
         }
      }
   }

   // Кнопки для меню снизу
   pub fn bottom_markup() -> ReplyKeyboardMarkup {
      ReplyKeyboardMarkup::default()
      .append_row(vec![
         KeyboardButton::new("В начало"),
         KeyboardButton::new("Добавить меню"),
      ])
      .resize_keyboard(true)
   }

}
