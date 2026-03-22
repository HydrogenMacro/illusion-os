import os
import sys
import subprocess


font_chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789.:"

def main():
    font_file = sys.argv[1]
    font_size = ""
    try:
        font_size = sys.argv[2]
    except:
        font_size = "80"
    

    font_path = "assets/fonts/" + font_file
    processed_font_path = f"assets/processed-fonts/{font_file}-{font_size}.bin"

    [font_width, font_height] = subprocess.run(f"magick -pointsize {font_size} -font {font_path} label:'a' -depth 8 png:- | magick identify -format '%w\n%h' png:-", shell=True, capture_output=True, text=True).stdout.split("\n")
    [font_width, font_height] = [int(font_width), int(font_height)]
    
    
    for char in font_chars:
        [char_width, char_height] = subprocess.run(f"magick -pointsize {font_size} -font {font_path} label:'a' -depth 8 png:- | magick identify -format '%w\n%h' png:-", shell=True, capture_output=True, text=True).stdout.split("\n")
        [char_width, char_height] = [int(char_width), int(char_height)]
        if char_width != font_width:
            raise ValueError(f"width of char '{char}' ({char_width}) does not match standard ({font_width})")
    
    print(f"font char size: {font_width}, {font_height}")
    
    try:
        os.remove(processed_font_path)
    except: pass
    with open(processed_font_path, "wb") as f:
        for (i, char) in enumerate(font_chars):
            char_img_data = subprocess.run(["magick", "-pointsize", font_size, "-fill", "white", "-background", "black", "-font", font_path, f"label:{char}",
                "-depth", "8", "gray:-"
            ], capture_output=True).stdout
            f.write(char_img_data)


    

if __name__ == "__main__":
    main()