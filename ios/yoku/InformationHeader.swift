//
//  InformationHeader.swift
//  yoku
//
//  Created by Alex Holder on 13/11/2025.
//

import SwiftUI

struct InformationHeader: View {
    var body: some View {
        /*
         - workout name
         - date (time?)
         - elapsed time
         - current exercise
         -
         */
        Grid {
            GridRow {
                HStack {
                    Text("SOME LOG").monospaced()
                    Text("VALUE").monospaced()
                }.padding(.vertical, 2)
                HStack {
                    Text("SOME LOG").monospaced()
                    Text("VALUE").monospaced()
                }.padding(0)
            }.padding(.horizontal)
            GridRow {
                HStack {
                    Text("SOME LOG").monospaced()
                    Text("VALUE").monospaced()
                }.padding(.vertical, 2)
                HStack {
                    Text("SOME LOG").monospaced()
                    Text("VALUE").monospaced()
                }.padding(0)
            }.padding(.horizontal)
            GridRow {
                HStack {
                    Text("SOME LOG").monospaced()
                    Text("VALUE").monospaced()
                }.padding(.vertical, 2)
                HStack {
                    Text("SOME LOG").monospaced()
                    Text("VALUE").monospaced()
                }.padding(0)
            }.padding(.horizontal)
        }.padding()
    }
}

#Preview {
    InformationHeader()
}
