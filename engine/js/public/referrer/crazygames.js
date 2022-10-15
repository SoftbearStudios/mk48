if (!('CrazyGames' in window)) {
    let script = document.createElement("script");
    script.setAttribute("src", "https://sdk.crazygames.com/crazygames-sdk-v1.js");
    script.onload = () => {
        function send(msg) {
            window.postMessage(msg, '*');
        }
        const sdk = window.CrazyGames.CrazySDK.getInstance();
        sdk.init();
        sdk.sdkGameLoadingStart();
        sdk.addEventListener("adStarted", () => {
            send('mute');
            send('pause');
        });
        sdk.addEventListener("adFinished", () => {
            send('unmute');
            send('unpause');
        });
        sdk.addEventListener("adError", () => {
            send('unmute');
            send('unpause');
        });
        sdk.addEventListener("bannerRendered", (event) => {
            console.log(`Banner for container ${event.containerId} has been rendered!`);
        });
        sdk.addEventListener("bannerError", (event) => {
            console.log(`Banner render error: ${event.error}`);
        });

        const requestBanner = (id, width, height) => {
            const container = document.getElementById(id);
            if (container) {
                container.style.width = `${width}px`;
                container.style.height = `${height}px`;
                sdk.requestBanner([
                    {
                        containerId: id,
                        size: `${width}x${height}`,
                    }
                ]);
            }
        };

        let first = true;
        window.addEventListener('message', (event) => {
            switch (event.data) {
                case "gameLoaded":
                    sdk.sdkGameLoadingStop();
                    break;
                case "splash":
                    if (first) {
                        first = false;
                        requestBanner("banner_bottom", 728, 90);
                    } else {
                        sdk.gameplayStop();
                        sdk.requestAd();
                    }
                    break;
                case "playing":
                    sdk.clearAllBanners();
                    sdk.gameplayStart();
                    break;
            }
        });
        send("snippetLoaded");
    };
    document.body.appendChild(script);
}