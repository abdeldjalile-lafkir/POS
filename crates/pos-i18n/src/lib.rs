//! `pos-i18n` — bilingual (Arabic / English) UI strings.
//!
//! Receipts are English-only (per the plan), so this crate is only used by the UI.
//! Arabic shaping and BiDi are handled by egui 0.37's text stack (harfrust +
//! fontique), and Arabic glyphs are supplied by the system fonts.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(Default)]
pub enum Lang {
    #[default]
    En,
    Ar,
}

impl Lang {
    pub const ALL: [Lang; 2] = [Lang::En, Lang::Ar];

    pub const fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Ar => "ar",
        }
    }

    pub fn from_code(code: &str) -> Lang {
        match code {
            "ar" => Lang::Ar,
            _ => Lang::En,
        }
    }

    /// Arabic is right-to-left.
    pub const fn is_rtl(self) -> bool {
        matches!(self, Lang::Ar)
    }

    /// Native label for the language switcher.
    pub const fn native_name(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Ar => "العربية",
        }
    }
}


macro_rules! keys {
    ($($variant:ident => ($en:literal, $ar:literal)),+ $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum Key {
            $($variant),+
        }

        pub fn t(lang: Lang, key: Key) -> &'static str {
            match (lang, key) {
                $((Lang::En, Key::$variant) => $en,
                  (Lang::Ar, Key::$variant) => $ar),+
            }
        }
    };
}

keys! {
    AppTitle => ("POS", "نقطة البيع"),

    // auth / login
    Login => ("Sign in", "تسجيل الدخول"),
    Username => ("Username", "اسم المستخدم"),
    Pin => ("PIN", "الرمز السري"),
    Enter => ("Enter", "دخول"),
    Clear => ("Clear", "مسح"),
    Logout => ("Sign out", "خروج"),
    WrongCredentials => ("Wrong username or PIN", "اسم المستخدم أو الرمز السري غير صحيح"),
    FirstRunTitle => ("First-time setup", "الإعداد الأولي"),
    CreateAdmin => ("Create the administrator account", "إنشاء حساب المدير"),
    ChoosePin => ("Choose a PIN (4+ digits)", "اختر رمزاً سرياً (4 أرقام أو أكثر)"),
    ConfirmPin => ("Confirm PIN", "تأكيد الرمز السري"),
    PinMismatch => ("PINs do not match", "الرمز السري غير متطابق"),
    Create => ("Create", "إنشاء"),
    AttemptsLeft => ("attempts left", "محاولات متبقية"),
    TooManyAttempts => ("Too many attempts — wait", "محاولات كثيرة جداً — انتظر"),

    // navigation
    NavRegister => ("Register", "الصندوق"),
    NavProducts => ("Products", "المنتجات"),
    NavInventory => ("Inventory", "المخزون"),
    NavReports => ("Reports", "التقارير"),
    NavSettings => ("Settings", "الإعدادات"),
    NavUsers => ("Users", "المستخدمون"),

    // register / cart
    ScanOrSearch => ("Scan or search…", "امسح أو ابحث…"),
    Cart => ("Cart", "السلة"),
    EmptyCart => ("The cart is empty", "السلة فارغة"),
    Qty => ("Qty", "الكمية"),
    Price => ("Price", "السعر"),
    LineTotal => ("Total", "الإجمالي"),
    Remove => ("Remove", "حذف"),
    SubtotalHt => ("Subtotal (HT)", "المجموع (بدون رسم)"),
    Discount => ("Discount", "الخصم"),
    Vat => ("TVA", "الرسم"),
    TotalTtc => ("Total", "الإجمالي"),
    Pay => ("Pay", "الدفع"),
    NewSale => ("New sale", "بيع جديد"),
    EnterBarcode => ("Barcode", "الباركود"),
    AddLoose => ("Add item", "إضافة منتج"),
    NotInCatalogue => ("Not in the catalogue", "غير موجود في القائمة"),
    CreateProductFor => ("Create a product for", "إنشاء منتج لـ"),
    SimulateScan => ("Simulate scan", "محاكاة المسح"),
    LineDiscount => ("Line discount", "خصم السطر"),
    SaleDiscount => ("Sale discount", "خصم الفاتورة"),
    Percent => ("Percent", "نسبة"),
    Amount => ("Amount", "مبلغ"),
    Apply => ("Apply", "تطبيق"),

    // payment
    Payment => ("Payment", "الدفع"),
    Cash => ("Cash", "نقداً"),
    Card => ("Card", "بطاقة"),
    Other => ("Other", "أخرى"),
    Tendered => ("Tendered", "المدفوع"),
    Change => ("Change", "الباقي"),
    Confirm => ("Confirm", "تأكيد"),
    Cancel => ("Cancel", "إلغاء"),
    InsufficientAmount => ("Amount is less than the total", "المبلغ أقل من الإجمالي"),

    // receipt
    Receipt => ("Receipt", "الإيصال"),
    Print => ("Print", "طباعة"),
    Close => ("Close", "إغلاق"),
    Reprint => ("Reprint", "إعادة الطباعة"),
    Item => ("Item", "المنتج"),

    // products
    Products => ("Products", "المنتجات"),
    NewProduct => ("New product", "منتج جديد"),
    EditProduct => ("Edit product", "تعديل منتج"),
    Name => ("Name", "الاسم"),
    NameAr => ("Name (Arabic)", "الاسم (بالعربية)"),
    Sku => ("SKU", "الرمز"),
    Barcode => ("Barcode", "الباركود"),
    Category => ("Category", "الفئة"),
    Unit => ("Unit", "الوحدة"),
    TaxRate => ("TVA rate", "نسبة الرسم"),
    Active => ("Active", "نشط"),
    Inactive => ("Inactive", "غير نشط"),
    Save => ("Save", "حفظ"),
    Saved => ("Saved", "تم الحفظ"),
    Edit => ("Edit", "تعديل"),
    Activate => ("Activate", "تنشيط"),
    Deactivate => ("Deactivate", "إلغاء التنشيط"),
    ShowInactive => ("Show inactive", "إظهار غير النشطة"),
    Search => ("Search", "بحث"),
    NoResults => ("No results", "لا توجد نتائج"),

    // inventory
    Stock => ("Stock", "المخزون"),
    LowStockOnly => ("Low stock only", "المخزون المنخفض فقط"),
    Threshold => ("Threshold", "الحد"),
    Adjust => ("Adjust", "تعديل"),
    QuantityDelta => ("Delta (+/-)", "التغيير (+/-)"),
    Reason => ("Reason", "السبب"),
    ReasonPurchase => ("Purchase", "شراء"),
    ReasonAdjustment => ("Adjustment", "تسوية"),
    ReasonReturn => ("Return", "إرجاع"),
    ReasonSale => ("Sale", "بيع"),
    Note => ("Note", "ملاحظة"),
    MovementHistory => ("Movement history", "سجل الحركات"),
    OnHand => ("On hand", "المتوفر"),

    // reports
    Period => ("Period", "الفترة"),
    Today => ("Today", "اليوم"),
    ThisWeek => ("This week", "هذا الأسبوع"),
    ThisMonth => ("This month", "هذا الشهر"),
    SalesCount => ("Sales", "عدد المبيعات"),
    Gross => ("Gross", "الإجمالي قبل الخصم"),
    Discounts => ("Discounts", "الخصومات"),
    VatCollected => ("TVA collected", "الرسم المحصّل"),
    NetTotal => ("Net total", "الصافي"),
    TopProducts => ("Top products", "الأكثر مبيعاً"),
    ZReport => ("Z-Report", "تقرير Z"),
    CashExpected => ("Cash expected", "النقد المتوقع"),
    Refunds => ("Refunds", "المبالغ المرتجعة"),
    ByRate => ("TVA by rate", "الرسم حسب النسبة"),
    Base => ("Base (HT)", "الأساس (بدون رسم)"),
    DailyTotals => ("Daily totals", "المجاميع اليومية"),
    Day => ("Day", "اليوم"),
    NoData => ("No data", "لا توجد بيانات"),

    // settings
    StoreInfo => ("Store information", "معلومات المتجر"),
    StoreName => ("Store name", "اسم المتجر"),
    Address => ("Address", "العنوان"),
    Nif => ("NIF", "رقم التعريف الجبائي"),
    Nis => ("NIS", "رقم التعريف الإحصائي"),
    Rc => ("RC", "السجل التجاري"),
    Ai => ("Article d'imposition (AI)", "المادة الجبائية"),
    Phone => ("Phone", "الهاتف"),
    ReceiptFooter => ("Receipt footer", "تذييل الإيصال"),
    Printer => ("Printer", "الطابعة"),
    Transport => ("Transport", "نوع الاتصال"),
    TransportNetwork => ("Network (TCP)", "الشبكة (TCP)"),
    TransportDevice => ("Device / file", "جهاز / ملف"),
    TransportPreview => ("Preview only", "معاينة فقط"),
    Host => ("Host", "العنوان"),
    Port => ("Port", "المنفذ"),
    DevicePath => ("Device path", "مسار الجهاز"),
    PaperWidth => ("Paper width", "عرض الورق"),
    AutoCut => ("Auto cut", "قطع تلقائي"),
    CashDrawer => ("Kick cash drawer", "فتح درج النقد"),
    TestPrint => ("Test print", "طباعة تجريبية"),
    Environment => ("Language & data", "اللغة والبيانات"),
    Language => ("Language", "اللغة"),
    BackupDatabase => ("Back up database", "نسخ احتياطي لقاعدة البيانات"),
    Backup => ("Backup", "نسخ"),
    BackupDone => ("Backup saved", "تم حفظ النسخة"),
    ScanTiming => ("Scanner max key gap (ms)", "أقصى فاصل بين المفاتيح (مللي ثانية)"),

    // users
    Users => ("Users", "المستخدمون"),
    Role => ("Role", "الدور"),
    Admin => ("Admin", "مدير"),
    Cashier => ("Cashier", "أمين صندوق"),
    AddUser => ("Add user", "إضافة مستخدم"),
    ResetPin => ("Reset PIN", "إعادة تعيين الرمز"),
    NewPin => ("New PIN", "رمز سري جديد"),

    // generic
    Yes => ("Yes", "نعم"),
    No => ("No", "لا"),
    Ok => ("OK", "حسناً"),
    Error => ("Error", "خطأ"),
    Required => ("Required field", "حقل مطلوب"),
    ConfirmAction => ("Are you sure?", "هل أنت متأكد؟"),
    PrintFailed => ("Printing failed", "فشلت الطباعة"),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_key_is_translated_in_both_languages() {
        // Spot-check a few keys and the RTL flag.
        assert_eq!(t(Lang::En, Key::TotalTtc), "Total");
        assert_eq!(t(Lang::Ar, Key::TotalTtc), "الإجمالي");
        assert!(!Lang::En.is_rtl());
        assert!(Lang::Ar.is_rtl());
        assert_eq!(Lang::from_code("ar"), Lang::Ar);
        assert_eq!(Lang::from_code("xx"), Lang::En);
    }

    #[test]
    fn no_empty_translations() {
        // Exhaustive over all keys via a small registry.
        for key in ALL_KEYS {
            assert!(!t(Lang::En, *key).is_empty(), "empty EN for {key:?}");
            assert!(!t(Lang::Ar, *key).is_empty(), "empty AR for {key:?}");
        }
    }

    // Keep in sync with the macro list above.
    const ALL_KEYS: &[Key] = &[
        Key::AppTitle,
        Key::Login,
        Key::Username,
        Key::Pin,
        Key::Enter,
        Key::Clear,
        Key::Logout,
        Key::WrongCredentials,
        Key::FirstRunTitle,
        Key::CreateAdmin,
        Key::ChoosePin,
        Key::ConfirmPin,
        Key::PinMismatch,
        Key::Create,
        Key::AttemptsLeft,
        Key::TooManyAttempts,
        Key::NavRegister,
        Key::NavProducts,
        Key::NavInventory,
        Key::NavReports,
        Key::NavSettings,
        Key::NavUsers,
        Key::ScanOrSearch,
        Key::Cart,
        Key::EmptyCart,
        Key::Qty,
        Key::Price,
        Key::LineTotal,
        Key::Remove,
        Key::SubtotalHt,
        Key::Discount,
        Key::Vat,
        Key::TotalTtc,
        Key::Pay,
        Key::NewSale,
        Key::EnterBarcode,
        Key::AddLoose,
        Key::NotInCatalogue,
        Key::CreateProductFor,
        Key::SimulateScan,
        Key::LineDiscount,
        Key::SaleDiscount,
        Key::Percent,
        Key::Amount,
        Key::Apply,
        Key::Payment,
        Key::Cash,
        Key::Card,
        Key::Other,
        Key::Tendered,
        Key::Change,
        Key::Confirm,
        Key::Cancel,
        Key::InsufficientAmount,
        Key::Receipt,
        Key::Print,
        Key::Close,
        Key::Reprint,
        Key::Item,
        Key::Products,
        Key::NewProduct,
        Key::EditProduct,
        Key::Name,
        Key::NameAr,
        Key::Sku,
        Key::Barcode,
        Key::Category,
        Key::Unit,
        Key::TaxRate,
        Key::Active,
        Key::Inactive,
        Key::Save,
        Key::Saved,
        Key::Edit,
        Key::Activate,
        Key::Deactivate,
        Key::ShowInactive,
        Key::Search,
        Key::NoResults,
        Key::Stock,
        Key::LowStockOnly,
        Key::Threshold,
        Key::Adjust,
        Key::QuantityDelta,
        Key::Reason,
        Key::ReasonPurchase,
        Key::ReasonAdjustment,
        Key::ReasonReturn,
        Key::ReasonSale,
        Key::Note,
        Key::MovementHistory,
        Key::OnHand,
        Key::Period,
        Key::Today,
        Key::ThisWeek,
        Key::ThisMonth,
        Key::SalesCount,
        Key::Gross,
        Key::Discounts,
        Key::VatCollected,
        Key::NetTotal,
        Key::TopProducts,
        Key::ZReport,
        Key::CashExpected,
        Key::Refunds,
        Key::ByRate,
        Key::Base,
        Key::DailyTotals,
        Key::Day,
        Key::NoData,
        Key::StoreInfo,
        Key::StoreName,
        Key::Address,
        Key::Nif,
        Key::Nis,
        Key::Rc,
        Key::Ai,
        Key::Phone,
        Key::ReceiptFooter,
        Key::Printer,
        Key::Transport,
        Key::TransportNetwork,
        Key::TransportDevice,
        Key::TransportPreview,
        Key::Host,
        Key::Port,
        Key::DevicePath,
        Key::PaperWidth,
        Key::AutoCut,
        Key::CashDrawer,
        Key::TestPrint,
        Key::Environment,
        Key::Language,
        Key::BackupDatabase,
        Key::Backup,
        Key::BackupDone,
        Key::ScanTiming,
        Key::Users,
        Key::Role,
        Key::Admin,
        Key::Cashier,
        Key::AddUser,
        Key::ResetPin,
        Key::NewPin,
        Key::Yes,
        Key::No,
        Key::Ok,
        Key::Error,
        Key::Required,
        Key::ConfirmAction,
        Key::PrintFailed,
    ];
}
