# Gathering the Magic

A utility for scanning, gathering, organizing, and maybe even analyzing your MTG card collection.

## About

This is a project that allows you to use your phone as a camera to scan MTG cards.

It'll then extract text from the card, search a database of MTG cards (which you'll need to download), and allows you to select which version of the card you want.

There's also a search function if it doesn't identify your card correctly.

In the end, you'll have a database.json containing which cards you chose and how many you have.

## How to use this project

Cards need to fill 20%-50% of the view area (you can change this, of course, in the source code - see `min_area` and `max_area` in `src/image_camera.rs`). On first run you'll need to download a database of the cards, this takes ~6h to respect the website that we're downloading from's rate limits. I recommend 3d printing a stand for your phone that will allow it to be parallel to the table without the legs of the stand getting in the way. About 12cm away from the table worked for me, with my phone, but you should do you own tests.

Otherwise, cloning this repo and running `cargo run --release` should mostly do it. Some browsers may only want to use https to work correctly. Press `ctrl-c` to kill the program whenever you're done. 

## How this project works

First it initializes a card database and will download high resolution images of all the cards from [Scryfall](https://scryfall.com/) (~3hrs to over-respect their rate limits..only runs once unless you delete the image files).

Then, it spins up a warp web server which will serve the card thumbnails, index.html, and manage a websocket connection.

The client sends frames at 5 fps over the websocket where I use tesseract to extract the text of the card (a process called OCR), we filter the space-separated tokens returned by the OCR against a list of all the space-separated tokens from the Scryfall database and rejoin it into a search phrase (all to filter junk from the OCR results). Finally we iterate all the cards, figure out the [Jaro Winkler](https://docs.rs/strsim/latest/strsim/fn.jaro_winkler.html) score for each field in the card for our query, return the max of those fields, and use that to return the top 30 cards.

The server will track the card position across frames and only research when a "new" card enters the camera field. Really, it can only track one card at a time and the card "dies" after it hasn't been seen for ~1.5s...this could have an effect on scanning speed, but when you select a card it "kills" the current one on the server. Pressing reject simply kills the card and forces OCR to run again...this is particularly useful if the card was eagerly identified but the capture was probably garbage because it was still moving or something.

Also, whenever you select a card in the UI it saves the card image id (because of cards that get reprinted) and its count. The completely history of modifications are stored too, actually. In case my jank code breaks, that database is always saved to disk and it creates a backup whenever it writes an update.

## Goal

The goal for this project is to take the database this process returns and write some simple [scry.rs](https://github.com/KyleMiles/scryrs) scripts...filtering the image file names out of the database to do some fancy queries to the Scryfall api and other websites...like calculating total collection value, or seeing if you have/how close you are to high-rated decks. I may or may not extend this project to include those things.
