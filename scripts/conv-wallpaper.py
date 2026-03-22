import subprocess
from itertools import batched
import sys
from array import array

def main():
    file_path = "assets/wallpapers/" + sys.argv[1]
    output_file_path = "assets/processed-wallpapers/" + sys.argv[1] + ".bin"
    (width, height) = (410, 502)
    rgb_data = list(subprocess.run(["magick", file_path, "-depth", "8", "-gravity", "center", "-resize", f"x{height}^", "-crop", f"{width}x{height}+0+0", "+repage", "rgb:-"], capture_output=True).stdout)
    rgb565_data = [(int(r*31/255)<<11)|(int(g*63/255)<<5)|int(b*31/255) for (r,g,b) in batched(rgb_data,3)]
    with open(output_file_path, "wb") as f:
        bin = array("H", rgb565_data) 
        if sys.byteorder == "little":
            bin.byteswap()
        f.write(bin.tobytes())        

if __name__ == "__main__":
    main()