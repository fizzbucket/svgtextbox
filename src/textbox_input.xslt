<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:xlink="http://www.w3.org/1999/xlink" xmlns:svg="http://www.w3.org/2000/svg" version="1.0" xmlns:dyn="http://exslt.org/dynamic">

	<xsl:template match="node()|@*">
		<xsl:copy>
			<xsl:apply-templates select="node()|@*"/>
	    </xsl:copy>
	</xsl:template>


	<xsl:template match="svg:textbox">
		<xsl:variable name="preceding_boxes" select="./preceding::svg:textbox"/>
		<xsl:element name="textbox" namespace="{namespace-uri()}">
			<xsl:attribute name="__id">
            	<xsl:text>textbox-</xsl:text><xsl:value-of select="count($preceding_boxes)"/>
            </xsl:attribute>
			<xsl:text>&#xA;{</xsl:text>
			<xsl:for-each select="./@*">
				<xsl:text>"</xsl:text>
				<xsl:value-of select="name()"/>
				<xsl:text>": </xsl:text>
				<xsl:choose>
					<xsl:when test="(normalize-space(.) != . or string(number(.)) = 'NaN' or (substring(. , string-length(.), 1) = '.') or (substring(., 1, 1) = '0') and not(. = '0')) and not(. = 'false') and not(. = 'true') and not(. = 'null')">
						<xsl:text>"</xsl:text>
							<xsl:value-of select="."/>
						<xsl:text>"</xsl:text>
					</xsl:when>
					<xsl:otherwise>
						<xsl:value-of select="."/>
					</xsl:otherwise>
				</xsl:choose>
				<xsl:text>,&#xA;</xsl:text>
			</xsl:for-each>
			<xsl:text>"markup": "</xsl:text><xsl:apply-templates select="./svg:markup/node()"/><xsl:text>"</xsl:text>
			<xsl:text>}</xsl:text>
		</xsl:element>
	</xsl:template>

	<xsl:template match="svg:markup//text()">
		<xsl:value-of select="normalize-space(.)" />
	</xsl:template>

	<xsl:template match="svg:preserved-space">
		<xsl:value-of select="' '" />
	</xsl:template>


	<xsl:template match="svg:br">
		<xsl:text>&#xA;</xsl:text>
	</xsl:template>

	<xsl:template match="svg:divider">
		<xsl:element name="span" namespace="{namespace-uri()}">
			<xsl:attribute name="font-family">
				<xsl:value-of select="'Spectral'"/>
			</xsl:attribute>
		<xsl:text>―――</xsl:text>
		</xsl:element>
	</xsl:template>

</xsl:stylesheet>
