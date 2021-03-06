pub const FONTS: [[u8; 16]; 256] = [
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [16, 16, 56, 56, 124, 124, 254, 254, 124, 124, 56, 56, 16, 16, 0, 0],
    [85, 170, 85, 170, 85, 170, 85, 170, 85, 170, 85, 170, 85, 170, 0, 0],
    [0, 136, 136, 136, 248, 136, 136, 136, 0, 62, 8, 8, 8, 8, 8, 8],
    [0, 248, 128, 128, 240, 128, 128, 128, 62, 32, 32, 60, 32, 32, 32, 0],
    [0, 112, 136, 128, 128, 128, 136, 112, 0, 60, 34, 34, 60, 40, 36, 34],
    [0, 128, 128, 128, 128, 128, 128, 248, 0, 62, 32, 32, 60, 32, 32, 32],
    [0, 0, 56, 68, 68, 68, 56, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 16, 16, 16, 16, 254, 16, 16, 16, 16, 0, 254, 0, 0, 0],
    [0, 132, 196, 164, 164, 148, 148, 140, 132, 32, 32, 32, 32, 32, 32, 62],
    [0, 0, 136, 136, 136, 80, 80, 32, 0, 62, 8, 8, 8, 8, 8, 0],
    [16, 16, 16, 16, 16, 16, 16, 16, 240, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 240, 16, 16, 16, 16, 16, 16, 16],
    [0, 0, 0, 0, 0, 0, 0, 0, 31, 16, 16, 16, 16, 16, 16, 16],
    [16, 16, 16, 16, 16, 16, 16, 16, 31, 0, 0, 0, 0, 0, 0, 0],
    [16, 16, 16, 16, 16, 16, 16, 16, 255, 16, 16, 16, 16, 16, 16, 16],
    [0, 0, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 255, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 0],
    [16, 16, 16, 16, 16, 16, 16, 16, 31, 16, 16, 16, 16, 16, 16, 16],
    [16, 16, 16, 16, 16, 16, 16, 16, 240, 16, 16, 16, 16, 16, 16, 16],
    [16, 16, 16, 16, 16, 16, 16, 16, 255, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 255, 16, 16, 16, 16, 16, 16, 16],
    [16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16],
    [0, 0, 2, 12, 48, 192, 48, 12, 2, 0, 254, 0, 254, 0, 0, 0],
    [0, 0, 0, 128, 96, 24, 6, 24, 96, 128, 254, 0, 254, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 254, 36, 36, 36, 36, 68, 132, 0, 0],
    [0, 0, 0, 0, 2, 4, 8, 254, 16, 254, 32, 64, 128, 0, 0, 0],
    [0, 0, 0, 0, 12, 18, 16, 16, 16, 124, 16, 16, 60, 82, 32, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 16, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 24, 24, 24, 24, 16, 16, 16, 16, 16, 16, 0, 0, 16, 16, 0],
    [108, 36, 36, 72, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 18, 18, 18, 127, 36, 36, 36, 36, 36, 254, 72, 72, 72, 72, 0],
    [16, 56, 84, 146, 146, 144, 80, 56, 20, 18, 146, 146, 84, 56, 16, 16],
    [1, 97, 146, 146, 148, 148, 104, 8, 16, 22, 41, 41, 73, 73, 134, 128],
    [0, 56, 68, 68, 68, 40, 16, 48, 74, 138, 132, 132, 74, 49, 0, 0],
    [96, 32, 32, 64, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 2, 4, 8, 8, 16, 16, 16, 16, 16, 16, 16, 8, 8, 4, 2],
    [0, 64, 32, 16, 16, 8, 8, 8, 8, 8, 8, 8, 16, 16, 32, 64],
    [0, 0, 0, 0, 16, 146, 84, 56, 84, 146, 16, 0, 0, 0, 0, 0],
    [0, 0, 0, 16, 16, 16, 16, 254, 16, 16, 16, 16, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 96, 32, 32, 64],
    [0, 0, 0, 0, 0, 0, 0, 254, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 96, 96, 0, 0],
    [0, 2, 2, 4, 4, 8, 8, 16, 16, 32, 32, 64, 64, 128, 128, 0],
    [0, 24, 36, 36, 66, 66, 66, 66, 66, 66, 66, 36, 36, 24, 0, 0],
    [0, 16, 16, 48, 80, 16, 16, 16, 16, 16, 16, 16, 16, 16, 0, 0],
    [0, 24, 36, 66, 66, 2, 4, 8, 16, 32, 32, 64, 64, 126, 0, 0],
    [0, 56, 68, 130, 130, 2, 4, 56, 4, 2, 130, 130, 68, 56, 0, 0],
    [0, 8, 24, 24, 40, 40, 72, 72, 136, 254, 8, 8, 8, 8, 0, 0],
    [0, 124, 64, 64, 64, 184, 196, 130, 2, 2, 130, 130, 68, 56, 0, 0],
    [0, 56, 68, 64, 128, 128, 184, 196, 130, 130, 130, 130, 68, 56, 0, 0],
    [0, 254, 2, 4, 4, 8, 8, 8, 8, 16, 16, 16, 16, 16, 16, 0],
    [0, 56, 68, 130, 130, 130, 68, 56, 68, 130, 130, 130, 68, 56, 0, 0],
    [0, 56, 68, 130, 130, 130, 130, 70, 58, 2, 2, 130, 68, 56, 0, 0],
    [0, 0, 0, 0, 24, 24, 0, 0, 0, 0, 0, 24, 24, 0, 0, 0],
    [0, 0, 0, 0, 24, 24, 0, 0, 0, 0, 24, 8, 8, 16, 0, 0],
    [0, 0, 0, 2, 4, 8, 16, 32, 32, 16, 8, 4, 2, 0, 0, 0],
    [0, 0, 0, 0, 0, 254, 0, 0, 0, 254, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 64, 32, 16, 8, 4, 4, 8, 16, 32, 64, 0, 0, 0],
    [0, 56, 68, 130, 130, 130, 4, 8, 8, 16, 16, 0, 0, 16, 16, 0],
    [0, 24, 36, 66, 90, 181, 165, 165, 165, 154, 64, 64, 34, 28, 0, 0],
    [0, 16, 16, 40, 40, 40, 68, 68, 68, 124, 130, 130, 130, 130, 0, 0],
    [0, 240, 136, 132, 132, 132, 136, 248, 132, 130, 130, 130, 132, 248, 0, 0],
    [0, 56, 68, 66, 128, 128, 128, 128, 128, 128, 128, 66, 68, 56, 0, 0],
    [0, 240, 136, 132, 132, 130, 130, 130, 130, 130, 132, 132, 136, 240, 0, 0],
    [0, 254, 128, 128, 128, 128, 128, 252, 128, 128, 128, 128, 128, 254, 0, 0],
    [0, 254, 128, 128, 128, 128, 128, 252, 128, 128, 128, 128, 128, 128, 0, 0],
    [0, 24, 36, 66, 64, 128, 128, 142, 130, 130, 130, 66, 102, 26, 0, 0],
    [0, 130, 130, 130, 130, 130, 130, 254, 130, 130, 130, 130, 130, 130, 0, 0],
    [0, 56, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 56, 0, 0],
    [0, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 66, 36, 24, 0, 0],
    [0, 66, 66, 68, 68, 72, 88, 104, 100, 68, 66, 66, 65, 65, 0, 0],
    [0, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 126, 0, 0],
    [0, 130, 130, 198, 198, 198, 170, 170, 170, 146, 146, 146, 146, 130, 0, 0],
    [0, 130, 194, 194, 162, 162, 146, 146, 146, 138, 138, 134, 134, 130, 0, 0],
    [0, 56, 68, 68, 130, 130, 130, 130, 130, 130, 130, 68, 68, 56, 0, 0],
    [0, 248, 132, 130, 130, 130, 132, 248, 128, 128, 128, 128, 128, 128, 0, 0],
    [0, 56, 68, 68, 130, 130, 130, 130, 130, 130, 186, 68, 68, 56, 8, 6],
    [0, 248, 132, 130, 130, 130, 132, 248, 136, 132, 132, 132, 130, 130, 0, 0],
    [0, 56, 68, 130, 130, 128, 96, 24, 4, 2, 130, 130, 68, 56, 0, 0],
    [0, 254, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 0, 0],
    [0, 130, 130, 130, 130, 130, 130, 130, 130, 130, 130, 130, 68, 56, 0, 0],
    [0, 130, 130, 130, 130, 68, 68, 68, 40, 40, 40, 16, 16, 16, 0, 0],
    [0, 146, 146, 146, 146, 146, 146, 170, 170, 108, 68, 68, 68, 68, 0, 0],
    [0, 130, 68, 68, 40, 40, 16, 40, 40, 40, 68, 68, 130, 130, 0, 0],
    [0, 130, 130, 68, 68, 68, 40, 40, 16, 16, 16, 16, 16, 16, 0, 0],
    [0, 254, 4, 4, 8, 8, 16, 16, 32, 32, 64, 64, 128, 254, 0, 0],
    [30, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 30],
    [0, 128, 128, 64, 64, 32, 32, 16, 16, 8, 8, 4, 4, 2, 2, 0],
    [240, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 240],
    [16, 40, 68, 130, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 254, 0],
    [48, 32, 32, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 60, 66, 2, 62, 66, 130, 130, 134, 122, 0, 0],
    [0, 128, 128, 128, 128, 184, 196, 130, 130, 130, 130, 130, 196, 184, 0, 0],
    [0, 0, 0, 0, 0, 56, 68, 130, 128, 128, 128, 130, 68, 56, 0, 0],
    [0, 2, 2, 2, 2, 58, 70, 130, 130, 130, 130, 130, 70, 58, 0, 0],
    [0, 0, 0, 0, 0, 56, 68, 130, 130, 254, 128, 130, 68, 56, 0, 0],
    [0, 12, 16, 16, 16, 124, 16, 16, 16, 16, 16, 16, 16, 16, 0, 0],
    [0, 0, 0, 0, 0, 59, 68, 68, 68, 56, 64, 120, 132, 130, 130, 124],
    [0, 64, 64, 64, 64, 92, 98, 66, 66, 66, 66, 66, 66, 66, 0, 0],
    [0, 16, 16, 0, 0, 48, 16, 16, 16, 16, 16, 16, 16, 16, 0, 0],
    [0, 8, 8, 0, 0, 24, 8, 8, 8, 8, 8, 8, 8, 8, 16, 96],
    [0, 64, 64, 64, 64, 66, 68, 72, 80, 104, 68, 68, 66, 66, 0, 0],
    [0, 48, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 0, 0],
    [0, 0, 0, 0, 0, 236, 146, 146, 146, 146, 146, 146, 146, 146, 0, 0],
    [0, 0, 0, 0, 0, 92, 98, 66, 66, 66, 66, 66, 66, 66, 0, 0],
    [0, 0, 0, 0, 0, 56, 68, 130, 130, 130, 130, 130, 68, 56, 0, 0],
    [0, 0, 0, 0, 0, 184, 196, 130, 130, 130, 130, 196, 184, 128, 128, 128],
    [0, 0, 0, 0, 0, 58, 70, 130, 130, 130, 130, 70, 58, 2, 2, 2],
    [0, 0, 0, 0, 0, 44, 48, 32, 32, 32, 32, 32, 32, 32, 0, 0],
    [0, 0, 0, 0, 0, 60, 66, 64, 96, 24, 6, 2, 66, 60, 0, 0],
    [0, 0, 16, 16, 16, 124, 16, 16, 16, 16, 16, 16, 16, 12, 0, 0],
    [0, 0, 0, 0, 0, 66, 66, 66, 66, 66, 66, 66, 70, 58, 0, 0],
    [0, 0, 0, 0, 0, 130, 130, 130, 68, 68, 40, 40, 16, 16, 0, 0],
    [0, 0, 0, 0, 0, 146, 146, 146, 146, 170, 170, 68, 68, 68, 0, 0],
    [0, 0, 0, 0, 0, 130, 68, 40, 40, 16, 40, 40, 68, 130, 0, 0],
    [0, 0, 0, 0, 0, 130, 130, 68, 68, 40, 40, 24, 16, 16, 32, 192],
    [0, 0, 0, 0, 0, 126, 4, 8, 8, 16, 16, 32, 64, 254, 0, 0],
    [4, 8, 8, 8, 8, 8, 8, 16, 8, 8, 8, 8, 8, 8, 8, 4],
    [16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16],
    [64, 32, 32, 32, 32, 32, 32, 16, 32, 32, 32, 32, 32, 32, 32, 64],
    [0, 0, 0, 96, 146, 12, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 32, 80, 80, 32, 0],
    [0, 62, 32, 32, 32, 32, 32, 32, 32, 32, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 16, 16, 16, 16, 16, 16, 16, 16, 240, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 32, 16, 16, 0],
    [0, 0, 0, 0, 0, 0, 0, 24, 24, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 126, 2, 2, 2, 126, 4, 4, 8, 8, 16, 32, 64, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 252, 36, 40, 48, 32, 64, 64, 128, 0],
    [0, 0, 0, 0, 0, 0, 8, 8, 16, 48, 80, 144, 16, 16, 16, 0],
    [0, 0, 0, 0, 0, 0, 32, 32, 252, 132, 132, 136, 8, 16, 32, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 124, 16, 16, 16, 16, 254, 0, 0],
    [0, 0, 0, 0, 0, 0, 8, 8, 254, 8, 24, 40, 72, 136, 24, 0],
    [0, 0, 0, 0, 0, 0, 64, 32, 62, 228, 40, 16, 16, 16, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 120, 8, 8, 8, 254, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 124, 4, 4, 124, 4, 4, 124, 0, 0],
    [0, 0, 0, 0, 0, 0, 36, 148, 84, 68, 8, 8, 16, 32, 0, 0],
    [0, 0, 0, 0, 0, 0, 60, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 254, 2, 18, 18, 20, 24, 16, 16, 32, 32, 64, 128, 0, 0],
    [0, 2, 2, 4, 4, 8, 24, 40, 72, 136, 8, 8, 8, 8, 0, 0],
    [0, 16, 16, 16, 254, 130, 130, 130, 130, 4, 4, 8, 16, 32, 0, 0],
    [0, 0, 0, 254, 16, 16, 16, 16, 16, 16, 16, 16, 255, 0, 0, 0],
    [0, 8, 8, 8, 254, 8, 24, 24, 40, 40, 72, 136, 8, 24, 0, 0],
    [0, 16, 16, 16, 254, 18, 18, 18, 18, 34, 34, 34, 66, 140, 0, 0],
    [0, 32, 32, 60, 224, 16, 16, 30, 240, 8, 8, 8, 8, 8, 0, 0],
    [0, 32, 32, 62, 34, 34, 66, 68, 132, 8, 8, 16, 32, 64, 0, 0],
    [0, 64, 64, 64, 126, 72, 72, 72, 136, 8, 16, 16, 32, 64, 0, 0],
    [0, 0, 0, 254, 2, 2, 2, 2, 2, 2, 2, 254, 2, 0, 0, 0],
    [0, 36, 36, 36, 36, 254, 36, 36, 36, 36, 4, 8, 16, 32, 0, 0],
    [0, 0, 96, 16, 0, 0, 194, 34, 4, 4, 8, 16, 32, 192, 0, 0],
    [0, 0, 0, 254, 4, 4, 4, 8, 8, 24, 20, 36, 66, 130, 0, 0],
    [0, 64, 64, 64, 64, 78, 114, 196, 72, 64, 64, 64, 64, 62, 0, 0],
    [0, 0, 2, 130, 66, 66, 66, 4, 4, 8, 8, 16, 32, 64, 0, 0],
    [0, 32, 32, 62, 34, 66, 98, 84, 140, 8, 8, 16, 32, 64, 0, 0],
    [0, 2, 12, 120, 8, 8, 255, 8, 8, 8, 8, 16, 32, 64, 0, 0],
    [0, 0, 32, 34, 146, 146, 66, 68, 4, 8, 8, 16, 32, 64, 0, 0],
    [0, 0, 126, 0, 0, 0, 254, 8, 8, 8, 8, 16, 32, 64, 0, 0],
    [0, 32, 32, 32, 32, 48, 40, 36, 34, 34, 32, 32, 32, 32, 0, 0],
    [0, 8, 8, 8, 254, 8, 8, 8, 8, 8, 16, 16, 32, 64, 0, 0],
    [0, 0, 0, 124, 0, 0, 0, 0, 0, 0, 0, 255, 0, 0, 0, 0],
    [0, 0, 126, 2, 2, 36, 20, 8, 12, 20, 18, 34, 64, 128, 0, 0],
    [16, 16, 16, 254, 4, 4, 8, 24, 52, 82, 146, 16, 16, 16, 0, 0],
    [0, 2, 2, 2, 4, 4, 4, 8, 8, 16, 32, 64, 128, 0, 0, 0],
    [0, 0, 8, 8, 36, 36, 36, 34, 34, 34, 34, 66, 66, 130, 0, 0],
    [0, 64, 64, 64, 66, 76, 112, 64, 64, 64, 64, 64, 64, 62, 0, 0],
    [0, 0, 254, 2, 2, 2, 2, 4, 4, 8, 8, 16, 32, 64, 0, 0],
    [0, 0, 32, 32, 80, 80, 136, 136, 4, 2, 1, 0, 0, 0, 0, 0],
    [0, 16, 16, 16, 254, 16, 24, 84, 84, 82, 82, 146, 16, 48, 0, 0],
    [0, 0, 0, 254, 2, 2, 4, 4, 200, 48, 16, 8, 4, 4, 0, 0],
    [0, 96, 24, 4, 0, 0, 96, 24, 4, 0, 224, 24, 4, 2, 0, 0],
    [0, 16, 16, 16, 16, 16, 16, 40, 36, 36, 34, 78, 242, 2, 0, 0],
    [0, 4, 4, 4, 4, 100, 24, 8, 12, 18, 16, 32, 64, 128, 0, 0],
    [0, 0, 254, 32, 32, 32, 254, 32, 32, 32, 32, 32, 30, 0, 0, 0],
    [0, 32, 32, 32, 23, 57, 210, 18, 12, 8, 8, 4, 4, 4, 0, 0],
    [0, 0, 0, 0, 124, 4, 4, 4, 4, 4, 4, 4, 255, 0, 0, 0],
    [0, 0, 254, 2, 2, 2, 2, 126, 2, 2, 2, 2, 2, 254, 0, 0],
    [0, 0, 124, 0, 0, 0, 254, 2, 2, 4, 4, 8, 16, 96, 0, 0],
    [0, 4, 68, 68, 68, 68, 68, 68, 68, 4, 8, 8, 16, 32, 0, 0],
    [0, 8, 8, 40, 40, 40, 40, 41, 41, 42, 74, 76, 72, 128, 0, 0],
    [0, 0, 64, 64, 64, 64, 64, 64, 66, 68, 68, 72, 80, 96, 0, 0],
    [0, 0, 0, 254, 130, 130, 130, 130, 130, 130, 130, 254, 130, 0, 0, 0],
    [0, 0, 0, 254, 130, 130, 130, 130, 2, 4, 4, 8, 16, 32, 0, 0],
    [0, 0, 0, 192, 32, 2, 2, 2, 4, 4, 8, 16, 32, 192, 0, 0],
    [0, 144, 72, 72, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 96, 144, 144, 96, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
];
