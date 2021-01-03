import 'package:flutter/material.dart';
import 'package:flutter_svg/svg.dart';
import 'package:tata_mobile/utils.dart';
import 'dart:math';

import 'package:theme_manager/theme_manager.dart';

class Welcome extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    final svgPath = ThemeManager.of(context).brightness == Brightness.dark
        ? "assets/svg/dark/phone_p2p.svg"
        : "assets/svg/phone_p2p.svg";
    return Container(
        color: Theme.of(context).backgroundColor,
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          crossAxisAlignment: CrossAxisAlignment.center,
          children: [
            Container(
              margin: EdgeInsets.fromLTRB(0, 0, 0, 20),
              child: SvgPicture.asset(svgPath),
            ),
            Text(
              "Welcome to Kittie chat",
              textAlign: TextAlign.center,
              style: Theme.of(context).textTheme.headline4,
            ),
            Text(
              "This is a secure p2p chat",
              style: Theme.of(context).textTheme.bodyText1,
            ),
            RaisedButton(
              onPressed: () => {},
              color: Theme.of(context).primaryColor,
              textColor: Colors.white,
              textTheme: ButtonTextTheme.primary,
              shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(18.0),
                  side: BorderSide(color: Theme.of(context).primaryColor)),
              child: Text("Next"),
            )
          ],
        ));
  }
}
