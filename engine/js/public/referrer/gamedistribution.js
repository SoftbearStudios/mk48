if (!("GD_OPTIONS" in window)) {
    function send(msg) {
        window.postMessage(msg, '*');
    }

    let first = true;
    let rewarded = false;

    const gameIds = {
        mk48:   "1802972a26fc4bcfa60a98c63ed86002",
        kiomet: "edfd4313006747cdb4ba38a74bb71f30",
    };

    const domainSegs = window.location.host.split('.');
    const gameName = domainSegs[domainSegs.length - 1];
    const gameId = gameIds[gameName] || gameIds.mk48;

    window.GD_OPTIONS = {
        gameId,
        onEvent: event => {
            console.log(`iframe received event: ${event.name}`);
            switch (event.name) {
                case 'SDK_GAME_START':
                    if (first) {
                        send("disableOutbound");
                        first = false;
                    }
                    send("unpause");
                    send("unmute");
                    break;
                case 'SDK_GAME_PAUSE':
                    send("pause");
                    send("mute");
                    break;
                case "ALL_ADS_COMPLETED":
                    if (rewarded) {
                        send("tallyRewardedAd");
                        rewarded = false;
                    } else {
                        send("tallyVideoAd");
                    }
                    break;
                case 'SDK_GDPR_TRACKING':
                    // NO-OP: No tracking relevant to GDPR to disable
                    break;
                case 'SDK_GDPR_TARGETING':
                    // NO-OP: No 3rd party advertisement services
                    break;
            }
        }
    };

    window.addEventListener('message', event => {
        //console.log(`iframe received message: ${event.data}`);
        switch (event.data) {
            case 'splash':
                console.log('iframe received message: splash');
                if (typeof gdsdk !== 'undefined' && gdsdk.showAd !== 'undefined') {
                    try {
                        rewarded = false;
                        gdsdk.showAd();
                    } catch (err) {
                        console.warn(err);
                    }
                }
                break;
            case 'requestRewardedAd':
                gdsdk.showAd("rewarded").then(() => {
                    rewarded = true;
                }).catch(() => {
                    send("cancelRewardedAd");
                });
                break;
        }
    }, false);

    (function(d, s, id) {
        var js, fjs = d.getElementsByTagName(s)[0];
        if (d.getElementById(id)) return;
        js = d.createElement(s);
        js.id = id;
        js.src = 'https://html5.api.gamedistribution.com/main.min.js';
        fjs.parentNode.insertBefore(js, fjs);
    }(document, 'script', 'gamedistribution-jssdk'));

    send("snippetLoaded");
    send("enableRewardedAds");
}