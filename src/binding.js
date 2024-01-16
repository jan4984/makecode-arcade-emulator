(function (THIZ) {
    const ffi = THIZ._engine;

    const SpriteKind = {
        create: () => ffi.sprite_kind_create(),
        Projectile: 2,
        Player: 1,
    };

    THIZ._221149842913key_events_cb = function (evt) {
        let ec = evt.split('(').map(v => v.replace(')', '')).filter(i => !!i);
        if (ec.length < 2) return;
        (controller.anyButton.fns[ec[0]] || []).forEach(f => f());
    }

    class Button {
        constructor(name) {
            this.name = name;
            this.fns = {};
        }
        onEvent(ev, f) {
            this.fns[ev] ??= [];
            this.fns[ev].push(f);
        }
    };

    const controller = {
        anyButton: new Button('*'),
    };

    const ControllerButtonEvent = {
        Pressed: 'KeyDown',
        Released: 'KeyUp',
    };

    const SpriteFlag = {
        AutoDestroy: 1,
        Invisible: 2,
    };

    class Animation {
        constructor(name) {
            this.name = name;
        }
        addAnimationFrame(img) {
            _log(`js:add frame to animation ${this.name}`);
            ffi.animation_add_frame(this.name, img.bmp);
        }
    }

    const ActionKind = {};

    const animation = {
        setAction(sprite, name) {
            name = ActionKind[name];
            //_log(`js:set action ${name}`)
            sprite.setAction(name);
        },
        createAnimation(name, interval) {            
            name = ActionKind[name];
            _log(`js:create animation ${name}`);
            ffi.animation_add(name, interval);
            return new Animation(name);
        },
        attachAnimation() {
            //animation is global, not need attach
        },
    }

    class Sprite {
        static pool = new Map();
        constructor(i) {
            this.i = i;
        }
        static _new(i){
            _log(`js:new sprite ${i}`);
            let ref = new Proxy(new Sprite(i), Sprite.proxyHandler);
            Sprite.pool.set(i, new WeakRef(ref));
            return ref;
        }
        static new(i) {
            let wr = Sprite.pool.get(i);
            if(!wr)
                return Sprite._new(i);
            let ref = wr.deref();
            if(!ref){
                Sprite.pool.delete(i);
                return Sprite._new(i);
            }
            return ref;
        }
        _getIdx() {
            return this.i;
        }
        startEffect() {
            //TODO:
        }
        setAction(v) {
            //_log(`set action of ${this.i} to ${v}`);
            ffi.sprite_active_action(this.i, v);
        }
        setFlag(f, tf) {
            ffi.sprite_set_flag(this.i, f, tf ? 1 : 0);
        }
        static proxyHandler = {
            get(sprite, name) {
                let i = sprite.i;
                if (name == "top" || name == "bottom" || name == 'left' || name == 'right') {
                    return ffi.sprite_get_bound(i, name);
                }
                let p = ["x", "y", "vx", "vy", "ax", "ay", "fx", "fy", "sx", "sy"].filter(v=>v == name);
                if (!p.length) {
                    return Reflect.get(...arguments);//throw `property ${name} of sprite not supported`;
                }
                //_log(`to get sprite property ${name}`);
                return ffi[`sprite_get_${name}`](i);
            },
            set(sprite, name, value) {
                let i = sprite.i;
                if (name == "top" || name == "bottom" || name == 'left' || name == 'right') {
                    ffi.sprite_set_bound(i, name, value);
                    return;
                }
                let p = ["x", "y", "vx", "vy", "ax", "ay", "fx", "fy", "sx", "sy"].filter(v=>v == name);
                if (!p.length) {
                    return Reflect.set(...arguments);//throw `property ${name} of sprite not supported`;
                }
                return ffi[`sprite_set_${name}`](i, value);
            }
        };
    };

    THIZ._221149842913overlap_cb = function (sp1, sp2, k1, k2) {
        const k = `${k1}-${k2}`;
        sprites._overlapCbs[k].forEach(f=>f(Sprite.new(sp1), Sprite.new(sp2)));
    };

    const sprites = {
        _overlapCbs: {},
        create: function (img, kind) {            
            let i = ffi.scene_add_sprite(img.bmp, kind);
            //_log(`sprite ${i} with bmp ${typeof(img.bmp)} ${img.bmp}`);
            return Sprite.new(i);
        },
        createProjectileFromSide(img, vx, vy) {
            let i = ffi.scene_add_sprite(img.bmp, SpriteKind.Projectile);
            let sp = Sprite.new(i);
            if (vx < 0) {
                sp.left = 160;
            } else {
                sp.right = 0;
            }
            if (vy < 0) {
                sp.bottom = 0;
            } else {
                sp.top = 120;
            }
            sp.vx = vx;
            sp.vy = vy;
            return sp;
        },
        onOverlap(k1, k2, f) {
            const k = `${k1}-${k2}`;
            if (!this._overlapCbs[k]) {
                this._overlapCbs[k] = [];
                ffi.scene_add_overlap_check_kinds(k1, k2);
            }
            this._overlapCbs[k].push(f);
        }
    };


    THIZ._221149842913game_loop = function (milliSec) {
        game._updateCbs.forEach(f => f());
        game._intervalCbs.forEach(item => {
            let { f, interval, elasped } = item;
            elasped += milliSec;
            //_log(`js:elasped ${elasped}`);
            let wait = interval - elasped;
            if (wait <= 0) {
                f();
                elasped = 0;
            }
            item.elasped = elasped;
        });
    };

    const game = {
        over(win) {
            ffi.game_over(win ? 1 : 0);
        },
        _updateCbs: [],
        onUpdate(f) {
            this._updateCbs.push(f);
        },
        _intervalCbs: [],
        onUpdateInterval(interval, f) {
            this._intervalCbs.push({
                f,
                interval,
                elasped: 0,
            })
        }
    };

    const scene = {
        setBackgroundColor(c) {
            ffi.scene_set_background_color(`${c}`);
        },
        screenHeight() {
            return 120;
        },
        screenWidth() {
            return 160;
        }
    };

    const info = {
        setScore: function (s) {
            //_log(`js: score ${s}`);
            ffi.info_set_score(s);
        },
        changeScoreBy: function (d) {
            //_log(`js: change score by ${d}`);
            ffi.info_change_score(d);
        }
    }

    class Effect {
        constructor(name) {
            this.name = name;
        }
        startScreenEffect() {
            ffi.scene_set_effect(this.name);
        }
    }

    const effects = {
        blizzard:new Effect("blizzard"),
    };

    function img(s) {
        return new Image(s[0]);
    }

    function randint(min, max) {
        return Math.floor(Math.random() * (max - min) + min);
    }

    class Image {
        constructor(bmp) {
            this.bmp = bmp;
        }
        fill(c) {
            this.bmp = this.bmp.replace(/[^\n]/g, c);
        }
    }

    const image = {
        create: function (w, h) {
            let line = '.';
            for (let x = 0; x < w; x++) {
                line += '.';
            }
            let lines = [];
            for (let y = 0; y < h; y++) {
                lines.push(line);
            }
            return new Image(lines.join('\n'));
        },
    };        

    Object.assign(THIZ, {
        img, image,
        sprites, SpriteKind,
        controller, ControllerButtonEvent,
        animation, info, game, scene, effects,
        randint,
        ActionKind, SpriteFlag
    });
})(globalThis);