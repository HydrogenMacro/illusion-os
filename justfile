convert-wallpaper file-name:
   uv run scripts/conv-assets.py {{file-name}}

upload-wallpaper:
    espflash write-bin 0x200000 assets/processed-wallpapers/test.png.bin