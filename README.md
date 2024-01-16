A demo libretro core to run makecode arcard project. It load and run .PNG project file saved from makecode. 

It's pure Rust but depends rusty_v8. No other c/c++ libraries need.

## to compile
download rust v8 from https://github.com/denoland/rusty_v8/releases v0.43.1 and place in in `rusty_v8_static_download/0.43.1`

## to run
With RetroArch, use manully scanning and set this core as default. https://arcade.makecode.com/06042-66270-49137-36569 tested.

## supported API
* img tag
* image.create(), image.fill()
* sprites.create(), sprites.createProjectileFromSide(), sprites.onOverlap()
* SpriteKind.Player, SpriteKind.create()
* controller.anyButton.onEvent(), ControllerButtonEvent.Pressed, ControllerButtonEvent.Released
* animation.createAnimation, animation.attachAnimation, animation.setAction(), Animation.addAnimationFrame()
* sprite.setAction(), sprite.setFlag(), sprite.[x,y,vx,vy,ax,ay,bottom,left,top,right]
* SpriteFlag.AutoDestroy, SpriteFlag.Invisible
* game.over(), game.onUpdate(), game.onUpdateInterval()
* scene.setBackgroundColor(), scene.screenHeight(), scene.screenWidth()
* info.setScore(), info.changeScoreBy()
* effects.bilizzard
* randint()