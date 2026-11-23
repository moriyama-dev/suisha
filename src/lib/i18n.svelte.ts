// English is the base language; Japanese is added on top.

const en = {
  'nav.updates': 'Updates',
  'nav.installed': 'Installed',
  'nav.search': 'Search',
  'nav.setup': 'Set up',
  'nav.history': 'History',
  'nav.report': 'Report a problem',
  'nav.request': 'Send a request',
  'nav.github': 'GitHub',
  'nav.language': 'Language',
  'nav.languageAuto': 'Automatic',

  'common.loading': 'Loading…',
  'common.retry': 'Try again',
  'common.cancel': 'Cancel',
  'common.save': 'Save',
  'common.close': 'Close',
  'common.busy': 'Another change is still running. Wait for it to finish.',
  'common.noManagers':
    'No supported package manager was found. Suisha works with apt, snap, Flatpak and Homebrew.',
  'common.error': 'Could not read from {manager}: {message}',

  'updates.title': 'Updates',
  'updates.check': 'Check again',
  'updates.checking': 'Checking for updates…',
  'updates.count': '{n} updates available',
  'updates.countOne': '1 update available',
  'updates.none': 'Everything is up to date.',
  'updates.updateAll': 'Update all',
  'updates.update': 'Update',
  'updates.listTitle': 'Waiting updates',
  'updates.heldTitle': 'Waiting for Ubuntu ({n})',
  'updates.heldText':
    'Ubuntu sends some updates to computers a little at a time (phased updates), and apt installs them when this computer’s turn comes. You do not need to do anything. “Kept back” means apt needs to change other packages first.',
  'updates.held.phased': 'Phased',
  'updates.held.kept-back': 'Kept back',
  'updates.checkedAt': 'Checked at {time}',
  'updates.updateSelected': 'Update selected ({n})',
  'updates.aptNote':
    'apt checks the package lists the system last downloaded. "Update all" downloads fresh lists first.',

  'auto.title': 'Automatic updates',
  'auto.on': 'On',
  'auto.off': 'Off',
  'auto.summaryDaily': 'Every day at {time}',
  'auto.summaryWeekly': 'Every {day} at {time}',
  'auto.summaryOff': 'Suisha does not update anything on its own.',
  'auto.change': 'Change schedule and apps',
  'auto.turnOn': 'Turn on',
  'auto.frequency': 'How often',
  'auto.daily': 'Every day',
  'auto.weekly': 'Once a week',
  'auto.day': 'Day',
  'auto.time': 'Time',
  'auto.what': 'What to update',
  'auto.notify': 'Show a notification when updates are installed or fail',
  'auto.explainLinux':
    'A system timer starts Suisha at that time, even when the window is closed. If the computer was off, it runs after the next login.',
  'auto.adminLinux':
    'To update apt and snap without anyone typing a password, Suisha asks for your password once when you turn this on. The permission covers only Suisha’s “update everything” helper.',
  'auto.adminAllowed': 'Password registered: apt and snap are updated automatically too.',
  'auto.saved': 'Automatic updates saved.',
  'auto.noHelpers':
    'apt and snap will not be updated automatically: install Suisha from its .deb package to enable this.',
  'auto.accessCancelled':
    'The password was not entered, so apt and snap will not be updated automatically. The other package managers will.',
  'auto.accessFailed': 'Could not allow password-less updates for apt and snap: {message}',
  'auto.lastRun': 'Last run: {time}',
  'auto.everyHour': 'Every hour',
  'auto.everyHours': 'Every {n} hours',
  'auto.hoursNote':
    'Package servers do not announce new versions, so Suisha checks on a schedule. Once an hour is the shortest interval, to keep the load on you and on the servers low.',
  'auto.next': 'Next: {time}',
  'auto.heroOff': 'Turn this on and Suisha keeps everything up to date by itself, even when its window is closed.',
  'auto.adminTitleNeeded': 'apt and snap need your password once',
  'auto.adminNeededText':
    'Until it is registered, automatic updates skip apt and snap. Suisha asks only once, and the permission covers only its “update everything” helper.',
  'auto.adminRegister': 'Register password',
  'auto.adminRegistered': 'Password registered. apt and snap are now updated automatically too.',
  'auto.saveWithPassword': 'Save and register password',
  'auto.passwordNext':
    'When you press the button below, the system asks for your password. Enter it so that apt and snap can be updated automatically.',
  'auto.sidebar': 'Automatic updates',
  'auto.closeOk': 'You can close this window. Suisha still updates in the background at these times.',

  'installed.title': 'Installed',
  'installed.filter': 'Filter by name',
  'installed.allManagers': 'All',
  'installed.count': '{n} apps',
  'installed.remove': 'Remove',
  'installed.confirmRemove': 'Remove?',
  'installed.note':
    'Shows what you installed yourself. Libraries pulled in as dependencies and support snaps are left out.',
  'installed.empty': 'Nothing here.',

  'search.title': 'Search',
  'search.placeholder': 'App name, for example firefox',
  'search.button': 'Search',
  'search.searching': 'Searching…',
  'search.none': 'Nothing found for “{q}”.',
  'search.install': 'Install',
  'search.installed': 'Installed',
  'search.hint': 'Suisha searches every package manager on this computer at once.',

  'setup.title': 'Set up',
  'setup.exportTitle': 'Save this computer’s apps',
  'setup.exportText':
    'Write the list of apps you installed to a file. Use it later to set up a new or reinstalled computer with the same operating system.',
  'setup.exportButton': 'Save list…',
  'setup.exported': 'Saved {n} apps to {path}.',
  'setup.importTitle': 'Set up from a saved list',
  'setup.importText':
    'Open a list saved by Suisha. Apps that are missing on this computer are installed in one go.',
  'setup.importButton': 'Open list…',
  'setup.fileFrom': 'List saved on {date}',
  'setup.selectMissing': 'Select all missing',
  'setup.selectNone': 'Select none',
  'setup.install': 'Install {n} apps',
  'setup.statusMissing': 'Not installed',
  'setup.statusInstalled': 'Already installed',
  'setup.statusUnavailable': '{manager} is not available here',
  'setup.statusInvalid': 'Invalid name',
  'setup.allPresent': 'Every app on the list is already installed.',
  'setup.tooLarge': 'The file is too large to be a Suisha list.',
  'setup.notASetupFile': 'This file is not a list saved by Suisha.',
  'setup.newerVersion': 'This list was saved by a newer version of Suisha. Update Suisha and try again.',
  'setup.otherOs':
    'This list was saved on {os}. Suisha opens only lists saved on the same kind of system, because the apps come from different places.',
  'setup.fileFilter': 'Suisha list',

  'history.title': 'History',
  'history.empty': 'Nothing has been changed yet.',
  'history.scheduled': 'Automatic',
  'history.manual': 'Manual',
  'history.ok': 'Done',
  'history.failed': 'Failed',
  'history.cancelled': 'Cancelled',
  'history.skipped': 'Skipped',
  'history.all': 'everything',
  'history.keep': 'Keep history for',
  'history.keepNone': 'Do not keep',
  'history.keepDays': '{n} days',
  'history.keepForever': 'Forever',
  'history.keepCustom': 'Other…',
  'history.keepCustomDays': 'Days (1 to {max})',
  'history.keepNote': 'Older entries are deleted automatically.',
  'history.keepNoneNote': 'Suisha writes no history. Failed automatic updates still show a notification.',
  'history.keepConfirm': 'This deletes {n} of the entries below. It cannot be undone.',
  'history.keepConfirmButton': 'Delete and change',
  'history.more': 'Show more',
  'history.adminSkipped':
    'apt and snap were skipped: they are not allowed to update without a password. Turn automatic updates off and on again, and enter your password when asked.',
  'action.install': 'Install',
  'action.remove': 'Remove',
  'action.update': 'Update',
  'action.update-all': 'Update all',

  'self.title': 'Suisha itself',
  'self.available': 'Suisha {v} is available',
  'self.upToDate': 'Suisha {v} is the latest version.',
  'self.current': 'You are using {v}.',
  'self.unknown': 'Could not check for a new version. Suisha tries again later.',
  'self.viaApt':
    'Suisha is also updated together with your other packages, by “Update all” and by automatic updates.',
  'self.updateNow': 'Update Suisha now',
  'self.updating': 'Updating Suisha',
  'self.updated': 'Suisha {v} is installed. Restart to start using it.',
  'self.restart': 'Restart Suisha',
  'self.check': 'Check now',
  'self.auto': 'Update Suisha automatically',
  'self.autoText':
    'Once a day, Suisha installs its own new version in the background. Turning this on asks for your password once; the permission covers only the helper that updates Suisha.',
  'self.autoOn': 'Suisha now updates itself once a day.',
  'self.autoCancelled': 'The password was not entered, so Suisha will not update itself automatically.',
  'self.manual':
    'This copy was not installed from the Suisha package, so it cannot update itself. Install the .deb package from the download page.',
  'self.download': 'Open the download page',
  'self.notes': 'What’s new',
  'self.button': 'Suisha {v}',

  'system.restartTitle': 'Restart to finish updating',
  'system.restartText': 'Ubuntu asks for a restart to finish installing these updates: {list}',
  'system.restartTextPlain': 'Ubuntu asks for a restart to finish installing updates.',
  'system.unneededTitle': '{n} packages are no longer needed',
  'system.unneededOne': '1 package is no longer needed',
  'system.unneededText':
    'They were installed for other packages that no longer use them (old kernels, for example): {list}',
  'system.cleanUp': 'Remove them',
  'system.flathubTitle': 'Flatpak has no app catalogue yet',
  'system.flathubText':
    'Flatpak is installed, but Flathub, where most Flatpak apps come from, is not set up. Adding it needs no password.',
  'system.addFlathub': 'Add Flathub',
  'system.more': 'and {n} more',
  'action.clean-up': 'Remove unneeded packages',
  'action.add-flathub': 'Add Flathub',

  'op.running': 'Working…',
  'op.done': 'Finished.',
  'op.failed': 'Finished with problems. See the details below.',
  'op.cancelled': 'Cancelled: the password was not entered.',
  'op.showLog': 'Show details',
  'op.hideLog': 'Hide details',
  'op.password': 'If the system asks for your password, enter it to continue.',

  'common.noManagersMac':
    'No supported package manager was found. Suisha works with Homebrew (install it from brew.sh) and, through the mas tool, the Mac App Store.',
  'auto.explainMac':
    'macOS starts Suisha at that time, even when the window is closed. If the Mac was asleep, it runs after it wakes up.',
  'auto.masNote':
    'Mac App Store apps are not included: the App Store updates them by itself when “Automatic Updates” is on in its settings.',
  'installed.noteMac':
    'Shows what you installed yourself. Formulae pulled in as dependencies are left out.',
  'installed.masRemove': 'Remove App Store apps from Finder or Launchpad.',
  'installed.masRemoveShort': 'Remove in Finder',
  'search.openAppStore': 'Open in App Store',
  'search.masNote': 'App Store apps are installed in the App Store: the button opens the app’s page there.',
  'updates.openAppStore': 'Open App Store',
  'updates.masNote':
    'Mac App Store apps are updated in the App Store. The button opens its Updates page; “Update all” opens it too.',
  'self.viaBrew':
    'Suisha is a Homebrew cask, so it is also updated together with your other apps, by “Update all” and by automatic updates.',
  'self.autoTextMac':
    'Once a day, Homebrew installs Suisha’s new version in the background. No password is needed.',
  'self.manualMac':
    'This copy was not installed with Homebrew, so it cannot update itself. Download the new disk image, or install Suisha with Homebrew to keep it up to date.',
  'self.noBrew': 'Install Suisha with Homebrew to let it update itself.',
  'os.macos': 'macOS',
  'os.linux': 'Linux',

  'day.0': 'Sunday',
  'day.1': 'Monday',
  'day.2': 'Tuesday',
  'day.3': 'Wednesday',
  'day.4': 'Thursday',
  'day.5': 'Friday',
  'day.6': 'Saturday',
} as const;

export type Key = keyof typeof en;

const ja: Record<Key, string> = {
  'nav.updates': '更新',
  'nav.installed': 'インストール済み',
  'nav.search': '検索',
  'nav.setup': 'セットアップ',
  'nav.history': '記録',
  'nav.report': '問題を報告',
  'nav.request': '要望を伝える',
  'nav.github': 'GitHub',
  'nav.language': '言語',
  'nav.languageAuto': '自動',

  'common.loading': '読み込み中…',
  'common.retry': 'もう一度',
  'common.cancel': 'キャンセル',
  'common.save': '保存',
  'common.close': '閉じる',
  'common.busy': 'ほかの処理がまだ動いています。終わるまでお待ちください。',
  'common.noManagers': '対応しているパッケージマネージャーが見つかりません。Suisha が扱えるのは apt、snap、Flatpak、Homebrew です。',
  'common.error': '{manager} から読み取れませんでした：{message}',

  'updates.title': '更新',
  'updates.check': 'もう一度確認',
  'updates.checking': '更新を確認しています…',
  'updates.count': '{n} 件の更新があります',
  'updates.countOne': '1 件の更新があります',
  'updates.none': 'すべて最新です。',
  'updates.updateAll': 'すべて更新',
  'updates.update': '更新',
  'updates.listTitle': '待っている更新',
  'updates.heldTitle': 'Ubuntu の配信待ち（{n} 件）',
  'updates.heldText':
    'Ubuntu は一部の更新を、少しずつ順番に配っています（段階的な配信）。このコンピューターの番が来たら、apt が自動で入れるので、何もしなくて大丈夫です。「保留中」は、ほかのパッケージを先に変える必要があるため、apt が後回しにしているものです。',
  'updates.held.phased': '段階的に配信中',
  'updates.held.kept-back': '保留中',
  'updates.checkedAt': '{time} に確認',
  'updates.updateSelected': '選んだものを更新（{n}）',
  'updates.aptNote':
    'apt は、システムが最後に取得したパッケージ一覧をもとに確認しています。「すべて更新」では、先に最新の一覧を取得します。',

  'auto.title': '自動更新',
  'auto.on': 'オン',
  'auto.off': 'オフ',
  'auto.summaryDaily': '毎日 {time}',
  'auto.summaryWeekly': '毎週{day} {time}',
  'auto.summaryOff': 'Suisha が自分で更新することはありません。',
  'auto.change': '頻度と対象を変更',
  'auto.turnOn': 'オンにする',
  'auto.frequency': '頻度',
  'auto.daily': '毎日',
  'auto.weekly': '週に1回',
  'auto.day': '曜日',
  'auto.time': '時刻',
  'auto.what': '更新するもの',
  'auto.notify': '更新したとき・失敗したときに通知する',
  'auto.explainLinux':
    'その時刻になると、システムのタイマーが Suisha を起動します。画面を閉じていても動きます。電源が切れていたときは、次にログインしたあとで動きます。',
  'auto.adminLinux':
    'apt と snap をパスワードの入力なしで更新できるように、オンにするときに1回だけパスワードを聞きます。許可されるのは Suisha の「すべて更新」の処理だけです。',
  'auto.adminAllowed': 'パスワード登録済み：apt と snap も自動で更新されます。',
  'auto.saved': '自動更新の設定を保存しました。',
  'auto.noHelpers':
    'apt と snap は自動では更新されません。使うには、Suisha を .deb パッケージから入れてください。',
  'auto.accessCancelled':
    'パスワードが入力されなかったため、apt と snap は自動では更新されません。ほかのパッケージマネージャーは自動で更新されます。',
  'auto.accessFailed': 'apt と snap をパスワードなしで更新する許可を設定できませんでした：{message}',
  'auto.lastRun': '前回：{time}',
  'auto.everyHour': '1 時間ごと',
  'auto.everyHours': '{n} 時間ごと',
  'auto.hoursNote':
    'パッケージの配信元は、新しい版が出たことを知らせてくれません。そのため Suisha は決まった間隔で確認します。このコンピューターと配信元の負担を抑えるため、いちばん短い間隔は 1 時間です。',
  'auto.next': '次回：{time}',
  'auto.heroOff': 'オンにすると、画面を閉じていても、Suisha がすべてを自動で最新に保ちます。',
  'auto.adminTitleNeeded': 'apt と snap には、パスワードの登録が1回だけ必要です',
  'auto.adminNeededText':
    '登録するまで、自動更新では apt と snap を飛ばします。パスワードを聞くのは1回だけで、許可されるのは Suisha の「すべて更新」の処理だけです。',
  'auto.adminRegister': 'パスワードを登録',
  'auto.adminRegistered': 'パスワードを登録しました。これで apt と snap も自動で更新されます。',
  'auto.saveWithPassword': '保存してパスワードを登録',
  'auto.passwordNext':
    '下のボタンを押すと、パスワードの入力画面が出ます。apt と snap を自動で更新できるように、入力してください。',
  'auto.sidebar': '自動更新',
  'auto.closeOk': 'この画面は閉じてかまいません。閉じていても、決めた時刻に Suisha が裏で更新します。',

  'installed.title': 'インストール済み',
  'installed.filter': '名前で絞り込む',
  'installed.allManagers': 'すべて',
  'installed.count': '{n} 件',
  'installed.remove': '削除',
  'installed.confirmRemove': '削除する？',
  'installed.note':
    '自分で入れたものだけを表示しています。依存関係で入ったライブラリや、snap の土台になるパッケージは表示していません。',
  'installed.empty': '何もありません。',

  'search.title': '検索',
  'search.placeholder': 'アプリの名前（例：firefox）',
  'search.button': '検索',
  'search.searching': '検索しています…',
  'search.none': '「{q}」に一致するものはありませんでした。',
  'search.install': 'インストール',
  'search.installed': 'インストール済み',
  'search.hint': 'このコンピューターにあるパッケージマネージャーをまとめて検索します。',

  'setup.title': 'セットアップ',
  'setup.exportTitle': 'このコンピューターのアプリを保存',
  'setup.exportText':
    '自分で入れたアプリの一覧をファイルに書き出します。同じ OS の新しいコンピューターや、入れ直したコンピューターのセットアップに使えます。',
  'setup.exportButton': '一覧を保存…',
  'setup.exported': '{n} 件のアプリを {path} に保存しました。',
  'setup.importTitle': '保存した一覧からセットアップ',
  'setup.importText':
    'Suisha で保存した一覧を開きます。このコンピューターに入っていないアプリを、まとめてインストールします。',
  'setup.importButton': '一覧を開く…',
  'setup.fileFrom': '{date} に保存した一覧',
  'setup.selectMissing': '未インストールをすべて選ぶ',
  'setup.selectNone': '選択を外す',
  'setup.install': '{n} 件をインストール',
  'setup.statusMissing': '未インストール',
  'setup.statusInstalled': 'インストール済み',
  'setup.statusUnavailable': 'ここには {manager} がありません',
  'setup.statusInvalid': '名前が正しくありません',
  'setup.allPresent': '一覧のアプリはすべてインストール済みです。',
  'setup.tooLarge': 'ファイルが大きすぎます。Suisha の一覧ではないようです。',
  'setup.notASetupFile': 'このファイルは Suisha で保存した一覧ではありません。',
  'setup.newerVersion': '新しい版の Suisha で保存された一覧です。Suisha を更新してから開いてください。',
  'setup.otherOs': 'この一覧は {os} で保存されたものです。アプリの入手先が違うため、Suisha で開けるのは同じ種類のシステムで保存した一覧だけです。',
  'setup.fileFilter': 'Suisha の一覧',

  'history.title': '記録',
  'history.empty': 'まだ何も変更していません。',
  'history.scheduled': '自動',
  'history.manual': '手動',
  'history.ok': '完了',
  'history.failed': '失敗',
  'history.cancelled': 'キャンセル',
  'history.skipped': 'スキップ',
  'history.all': 'すべて',
  'history.keep': '記録を残す期間',
  'history.keepNone': '残さない',
  'history.keepDays': '{n} 日',
  'history.keepForever': 'ずっと残す',
  'history.keepCustom': '日数を指定…',
  'history.keepCustomDays': '日数（1〜{max}）',
  'history.keepNote': 'これより古い記録は自動で削除されます。',
  'history.keepNoneNote': '記録は残しません。自動更新が失敗したときは、これまでどおり通知でお知らせします。',
  'history.keepConfirm': '下の記録のうち {n} 件が削除されます。元に戻せません。',
  'history.keepConfirmButton': '削除して変更',
  'history.more': 'さらに表示',
  'history.adminSkipped':
    'apt と snap は飛ばしました。パスワードなしで更新する許可がありません。自動更新をいったんオフにしてからオンにし、パスワードを求められたら入力してください。',
  'action.install': 'インストール',
  'action.remove': '削除',
  'action.update': '更新',
  'action.update-all': 'すべて更新',

  'self.title': 'Suisha 自体',
  'self.available': 'Suisha {v} があります',
  'self.upToDate': 'Suisha {v} は最新です。',
  'self.current': '今の版は {v} です。',
  'self.unknown': '新しい版があるか確認できませんでした。あとでもう一度確認します。',
  'self.viaApt': 'Suisha は、「すべて更新」や自動更新でも、ほかのパッケージと一緒に更新されます。',
  'self.updateNow': '今すぐ Suisha を更新',
  'self.updating': 'Suisha の更新',
  'self.updated': 'Suisha {v} を入れました。再起動すると新しい版になります。',
  'self.restart': 'Suisha を再起動',
  'self.check': '今すぐ確認',
  'self.auto': 'Suisha を自動で更新する',
  'self.autoText':
    '1日に1回、Suisha が自分の新しい版を裏で入れます。オンにするときに1回だけパスワードを聞きます。許可されるのは、Suisha を更新する処理だけです。',
  'self.autoOn': 'これから Suisha は、1日に1回、自動で更新されます。',
  'self.autoCancelled': 'パスワードが入力されなかったため、Suisha の自動更新はオンにしていません。',
  'self.manual':
    'この Suisha は .deb パッケージから入れたものではないので、自分では更新できません。ダウンロードページから .deb パッケージを入れてください。',
  'self.download': 'ダウンロードページを開く',
  'self.notes': '変更点',
  'self.button': 'Suisha {v}',

  'system.restartTitle': '更新を完了するには再起動が必要です',
  'system.restartText': '次の更新を完了するために、Ubuntu が再起動を求めています：{list}',
  'system.restartTextPlain': '更新を完了するために、Ubuntu が再起動を求めています。',
  'system.unneededTitle': '不要になったパッケージが {n} 件あります',
  'system.unneededOne': '不要になったパッケージが 1 件あります',
  'system.unneededText':
    'ほかのパッケージのために入ったもので、今はどこからも使われていません（古いカーネルなど）：{list}',
  'system.cleanUp': '削除する',
  'system.flathubTitle': 'Flatpak にアプリの一覧がまだありません',
  'system.flathubText':
    'Flatpak は入っていますが、ほとんどの Flatpak アプリの配布元である Flathub が未設定です。追加にパスワードは要りません。',
  'system.addFlathub': 'Flathub を追加',
  'system.more': 'ほか {n} 件',
  'action.clean-up': '不要なパッケージを削除',
  'action.add-flathub': 'Flathub を追加',

  'op.running': '処理しています…',
  'op.done': '完了しました。',
  'op.failed': '一部がうまくいきませんでした。下の詳細を確認してください。',
  'op.cancelled': 'パスワードが入力されなかったため、キャンセルしました。',
  'op.showLog': '詳細を表示',
  'op.hideLog': '詳細を隠す',
  'op.password': 'パスワードを求められたら、入力すると続きが始まります。',

  'common.noManagersMac':
    '対応しているパッケージマネージャーが見つかりません。Suisha が扱えるのは Homebrew（brew.sh から入れられます）と、mas というツールを通した Mac App Store です。',
  'auto.explainMac':
    'その時刻になると、macOS が Suisha を起動します。画面を閉じていても動きます。Mac がスリープ中だったときは、復帰したあとで動きます。',
  'auto.masNote':
    'Mac App Store のアプリは含めません。App Store の設定で「自動アップデート」をオンにすると、App Store が自分で更新します。',
  'installed.noteMac':
    '自分で入れたものだけを表示します。依存関係として一緒に入った formula は表示しません。',
  'installed.masRemove': 'App Store のアプリは Finder か Launchpad から削除してください。',
  'installed.masRemoveShort': 'Finder で削除',
  'search.openAppStore': 'App Store で開く',
  'search.masNote': 'App Store のアプリは App Store でインストールします。ボタンを押すと、そのアプリのページが開きます。',
  'updates.openAppStore': 'App Store を開く',
  'updates.masNote':
    'Mac App Store のアプリは App Store で更新します。ボタンを押すと App Store の「アップデート」が開きます。「すべて更新」でも開きます。',
  'self.viaBrew':
    'Suisha は Homebrew の cask なので、「すべて更新」や自動更新でも、ほかのアプリと一緒に更新されます。',
  'self.autoTextMac':
    '1日1回、Homebrew が Suisha の新しい版を裏で入れます。パスワードは要りません。',
  'self.manualMac':
    'このコピーは Homebrew で入れたものではないため、自分では更新できません。新しいディスクイメージをダウンロードするか、Homebrew で入れ直すと、自動で最新に保てます。',
  'self.noBrew': 'Suisha が自分で更新するには、Homebrew で Suisha を入れてください。',
  'os.macos': 'macOS',
  'os.linux': 'Linux',

  'day.0': '日曜日',
  'day.1': '月曜日',
  'day.2': '火曜日',
  'day.3': '水曜日',
  'day.4': '木曜日',
  'day.5': '金曜日',
  'day.6': '土曜日',
};

export type Lang = 'en' | 'ja';
export type LangChoice = Lang | 'auto';

const STORAGE_KEY = 'suisha.language';

function systemLang(): Lang {
  const langs = navigator.languages?.length ? navigator.languages : [navigator.language];
  return langs.some((l) => l?.toLowerCase().startsWith('ja')) ? 'ja' : 'en';
}

function readChoice(): LangChoice {
  try {
    const v = localStorage.getItem(STORAGE_KEY);
    if (v === 'en' || v === 'ja' || v === 'auto') return v;
  } catch {
    // Storage may be unavailable; fall back to automatic.
  }
  return 'auto';
}

export const i18n = $state({ choice: readChoice() as LangChoice, lang: 'en' as Lang });
i18n.lang = i18n.choice === 'auto' ? systemLang() : i18n.choice;

export function setLanguage(choice: LangChoice) {
  i18n.choice = choice;
  i18n.lang = choice === 'auto' ? systemLang() : choice;
  try {
    localStorage.setItem(STORAGE_KEY, choice);
  } catch {
    // Not fatal: the choice just will not be remembered.
  }
  document.documentElement.lang = i18n.lang;
}

export function t(key: Key, params: Record<string, string | number> = {}): string {
  const table: Record<string, string> = i18n.lang === 'ja' ? ja : en;
  let text = table[key] ?? en[key] ?? key;
  for (const [k, v] of Object.entries(params)) {
    text = text.replaceAll(`{${k}}`, String(v));
  }
  return text;
}

/** Translate a message key sent by the Rust side ("key" or "key:detail"). */
export function tMessage(message: string, extra: Record<string, string> = {}): string {
  const [key, ...rest] = message.split(':');
  const detail = rest.join(':');
  if (key in en) {
    return t(key as Key, { message: detail, os: detail ? t(`os.${detail}` as Key) : '', ...extra });
  }
  return message;
}
