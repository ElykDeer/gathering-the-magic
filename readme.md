# Gathering the Magic

A utility for scanning, gathering, organizing, and maybe even analyzing your MTG card collection.

## About

This is a project that allows you to use your phone as a camera to scan MTG cards.

It'll then extract text from the card, search a database of MTG cards (which you'll need to download), and allows you to select which version of the card you want.

There's also a search function if it doesn't identify your card correctly.

In the end, you'll have a database.json containing which cards you chose and how many you have.

## Will this project work for me?

Probably not, sorry. But, mainly because there are some constants in the code I need to expose as variables. For example, I hard coded the index of the camera I want to use in the HTML. The size of cards is also hard-coded, and will vary based on resolution and distance to the cards.

If you want to try using this project please [reach out to me](elyk.dev) to let me know so I can make this project more user friendly.

But in theory if you happen to have a Samsung S23+, can host this code with https at laptop.elyk.io, and can vertically mount your phone 13cm parallel to a table (I recommend 3d printing something...mine took a little less than 3h to print), it should work.

## How this project works

First it initializes a card database and will download high resolution images of all the cards from [Scryfall](https://scryfall.com/) (~3hrs to over-respect their rate limits..only runs once unless you delete the image files).

Then, it spins up a warp web server which will serve the card thumbnails, index.html, and manage a websocket connection.

The client sends frames at 5 fps over the websocket where I use tesseract to extract the text of the card (a process called OCR), we filter the space-separated tokens returned by the OCR against a list of all the space-separated tokens from the Scryfall database and rejoin it into a search phrase (all to filter junk from the OCR results). Finally we iterate all the cards, figure out the [Jaro Winkler](https://docs.rs/strsim/latest/strsim/fn.jaro_winkler.html) score for each field in the card for our query, return the max of those fields, and use that to return the top 30 cards.

The server will track the card position across frames and only research when a "new" card enters the camera field. Really, it can only track one card at a time and the card "dies" after it hasn't been seen for ~1.5s...this could have an effect on scanning speed, but when you select a card it "kills" the current one on the server. Pressing reject simply kills the card and forces OCR to run again...this is particularly useful if the card was eagerly identified but the capture was probably garbage because it was still moving or something.

Also, whenever you select a card in the UI it saves the card image id (because of cards that get reprinted) and its count. The completely history of modifications are stored too, actually. In case my jank code breaks, that database is always saved to disk and it creates a backup whenever it writes an update.

## Goal

The goal for this project is to take the database this process returns and write some simple [scry.rs](https://github.com/KyleMiles/scryrs) scripts...filtering the image file names out of the database to do some fancy queries to the Scryfall api and other websites...like calculating total collection value, or seeing if you have/how close you are to high-rated decks. I may or may not extend this project to include those things.
