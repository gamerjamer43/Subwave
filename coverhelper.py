from mutagen.flac import FLAC, Picture
from mutagen.oggvorbis import OggVorbis
from mutagen.wave import WAVE
from mutagen.mp3 import MP3
from mutagen.id3._frames import APIC, TIT2, TPE1, TPE2, TALB, TDRC
import os

def make_picture(cover_data: bytes, mime_type: str) -> Picture:
    """make a Picture obj for flac/ogg cover art.
    
    Args:
        cover_data: raw image bytes
        mime_type: mime type of the image (e.g. "image/png")
        
    Returns:
        Picture obj w all the metadata set
    """
    pic = Picture()
    pic.data = cover_data
    pic.type = 3  # front cover
    pic.mime = mime_type
    pic.desc = "Cover"
    return pic


def add_id3_tags(audio, cover_data: bytes, mime_type: str, title: str, 
                 artists: str, contributing_artists: str, album: str, year: str) -> bool:
    """add id3 tags to mp3/wav. exits if tags already exist.
    
    Args:
        audio: mutagen audio object (MP3 or WAVE)
        cover_data: raw image bytes
        mime_type: mime type of the image
        title: song title
        artists: main artists
        contributing_artists: album artists
        album: album name
        year: release year
        
    Returns:
        True if tags were added, False if they already existed
    """
    # if tags already exist, print em and bail
    if audio.tags is not None:
        print("\033[31m[!] file already has tags!")
        print("existing tags:")
        if audio.tags.get('TIT2'): print(f"  - title: {audio.tags.get('TIT2').text[0]}")
        if audio.tags.get('TPE1'): print(f"  - artist: {audio.tags.get('TPE1').text[0]}")
        if audio.tags.get('TPE2'): print(f"  - album artist: {audio.tags.get('TPE2').text[0]}")
        if audio.tags.get('TALB'): print(f"  - album: {audio.tags.get('TALB').text[0]}")
        if audio.tags.get('TDRC'): print(f"  - year: {audio.tags.get('TDRC').text[0]}")
        print("\033[0m")
        return False
    
    # create tags and add all metadata
    audio.add_tags()
    audio.tags.add(APIC(encoding=3, mime=mime_type, type=3, desc="Cover", data=cover_data))
    audio.tags.add(TIT2(encoding=3, text=title))
    audio.tags.add(TPE1(encoding=3, text=artists))
    audio.tags.add(TPE2(encoding=3, text=contributing_artists))
    audio.tags.add(TALB(encoding=3, text=album))
    audio.tags.add(TDRC(encoding=3, text=year))
    return True


def process_audio(audio_file: str, cover_file: str, title: str, artists: str, 
                  contributing_artists: str, album: str, year: str) -> bool:
    """process audio file and add metadata + cover art.
    
    Args:
        audio_file: path to audio file
        cover_file: path to cover image
        title: song title
        artists: main artists
        contributing_artists: album artists
        album: album name
        year: release year
        
    Returns:
        True if successful, False otherwise
    """
    # get file ext
    audio_ext: str = os.path.splitext(audio_file)[1].lower()
    cover_ext: str = os.path.splitext(cover_file)[1].lower()
    
    # figure out mime type frm the file ext
    mime_type: str = f"image/{cover_ext[1:]}" if cover_ext in [".jpg", ".jpeg", ".png", ".gif", ".bmp", ".webp"] else "image/jpeg"
    
    # load cover image data
    with open(cover_file, "rb") as img:
        cover_data = img.read()
    
    # flac/ogg use vorbis comments
    if audio_ext in [".flac", ".ogg"]:
        audio = FLAC(audio_file) if audio_ext == ".flac" else OggVorbis(audio_file)
        
        # add cover art (works different between the twho)
        if audio_ext == ".flac":
            audio.add_picture(make_picture(cover_data, mime_type)) # type: ignore

        elif audio_ext == ".ogg":
            audio["metadata_block_picture"] = [make_picture(cover_data, mime_type)]
        
        # add metadata (key value pairs as god intended)
        audio["title"] = title
        audio["artist"] = artists
        audio["albumartist"] = contributing_artists
        audio["album"] = album
        audio["date"] = year
        audio.save()
        return True
        
    # mp3/wav use id3 tags
    elif audio_ext in [".wav", ".mp3"]:
        audio = MP3(audio_file) if audio_ext == ".mp3" else WAVE(audio_file)
        success = add_id3_tags(audio, cover_data, mime_type, title, artists, contributing_artists, album, year)

        if success:
            audio.save()
        return success
        
    else:
        print(f"unsupported file format: {audio_ext}")
        return False


def main() -> None:
    """main loop."""
    while True:
        print("\n" + "="*50)
        print("audio metadata & cover art tool")
        print("="*50)
        
        # get all inputs
        audio_file: str = input("audio file: ")
        cover_file: str = input("cover art: ")
        title: str = input("title: ")
        artists : str= input("artists (comma-separated): ")
        contributing_artists: str = input("contributing artists (blank = same as artists): ").strip()
        if not contributing_artists:
            contributing_artists = artists
        album: str = input("album: ")
        year: str = input("year: ")
        
        # process
        if process_audio(audio_file, cover_file, title, artists, contributing_artists, album, year):
            print("\033[32m[✓] done!\033[0m")
        
        # continue?
        if input("\nanother? (y/n): ").strip().lower() != 'y':
            break
    
    print("\nexiting!")

if __name__ == "__main__":
    main()