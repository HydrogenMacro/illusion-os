convert-wallpaper img-file:
   uv run scripts/conv-wallpaper.py {{img-file}}

upload-wallpaper img-file:
   espflash write-bin 0x110000 ./assets/processed-wallpapers/{{img-file}}.bin

process-font font-file font-size:
   uv run scripts/conv-fonts.py {{font-file}} {{font-size}}

upload-font font-file font-size offset:
   espflash write-bin {{offset}} ./assets/processed-fonts/{{font-file}}-{{font-size}}.bin
