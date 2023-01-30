# MainFrame
### System zapisu i udostępniania plików

<br>

### Opis
&nbsp;&nbsp;&nbsp;&nbsp;MainFrame służy do zapisu plików z możliwością ich obejrzenia, zmienienia nazwy, pobrania czy też udostępnienia innym użytkownikom. Korzystanie z funkcjonalności systemu wymaga założenia konta i zalogowania się. Hasła dla dodatkowej ochrony są hashowane z solą algorytmem SHA512, a sama sesja logowania zapisywana jest w plikach cookies.

&nbsp;&nbsp;&nbsp;&nbsp;System jest aplikacją internetową zaprogramowaną w języku *Rust* z framework-iem do aplikacji webowych *Rocket* i wykorzystuje takie technologie jak:
 - zapis informacji do bazy danych SQLite
 - zapis danych do pliku CSV (logi)
 - przedstawianie interfejsu za pomocą szablonowania Handlebars
 - zapis w plikach cookies przeglądarki (sesja logowania)

### Baza danych
&nbsp;&nbsp;&nbsp;&nbsp;Aplikacja korzysta z bazy danych SQLite, której plik załączony jest w kodzie źródłowym (plik MainFrame.db), a plik SQLite_Tables_Creation_String.txt przedstawia strukturę tabel w bazie, która dodatkowo jest przedstawiona poniżej.
```
CREATE TABLE Users(ID INTEGER PRIMARY KEY AUTOINCREMENT,
                   Username TEXT NOT NULL UNIQUE,
                   Hash TEXT NOT NULL,
                   Salt TEXT NOT NULL,
                   SessionID TEXT
                   );
                   
CREATE TABLE Files(ID INTEGER PRIMARY KEY AUTOINCREMENT,
                   Filename TEXT NOT NULL,
                   Content TEXT NOT NULL,
                   MimeType TEXT NOT NULL
                   );
                   
CREATE TABLE UserFiles (ID INTEGER PRIMARY KEY AUTOINCREMENT,
                       UserID INTEGER NOT NULL,
                       FileID INTEGER NOT NULL,
                       Owner INTEGER NOT NULL,
                       FOREIGN KEY(UserID) REFERENCES Users(ID),
                       FOREIGN KEY(FileID) REFERENCES Files(ID)
                       );
```

&nbsp;&nbsp;&nbsp;&nbsp;Tabela **Users** przechowuje informację co do nazwy użytkownika, jego hasła (za pomocą hashu i soli) oraz id sesji, która waliduje, czy użytkownik jest zalogowany.

&nbsp;&nbsp;&nbsp;&nbsp;**Files** zawiera nazwę pliku, sam plik w postaci heksadecymalnej oraz typ pliku, które są potrzebne podczas pokazywania tego pliku na interfejsie przeglądarki.

&nbsp;&nbsp;&nbsp;&nbsp;**UserFiles** przechowuje informację dotyczące praw do danego pliku poprzez samo ID pliku i użytkownika oraz zmienną Owner, przyjmującą wartości 0 i 1.

## Poprzednik
Projekt *MainFrame* to sukcesor starego projektu [*Czytadło*](https://github.com/Io-Maciek/Czytadlo) napisanego w roku 2020 w języku PHP.
